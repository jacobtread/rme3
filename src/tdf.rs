use std::any::Any;
use std::collections::HashMap;
use std::fs::read_to_string;
use std::io;
use std::io::{ErrorKind, Read};
use std::ops::Add;
use std::string::FromUtf8Error;

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

#[repr(u8)]
enum TdfType {
    VarInt = 0x0,
    String = 0x1,
    Blob = 0x2,
    Group = 0x3,
    List = 0x4,
    Map = 0x5,
    Union = 0x6,
    VarIntList = 0x7,
    Pair = 0x8,
    Tripple = 0x9,
    Float = 0xA,
}


#[derive(PartialEq)]
enum MapKey {
    VarInt(u32),
    String(String),
}

#[derive(PartialEq)]
enum MapValue {
    VarInt(VarInt),
    String(String),
    Struct(Vec<Tdf>),
    Float(f32),
}

#[repr(u8)]
enum SubDataType {
    VarInt = 0x0,
    String = 0x1,
    Struct = 0x3,
    Tripple = 0x9,
    Float = 0xA,
}

pub struct VarInt(pub i64);

impl From<usize> for VarInt {
    fn from(value: usize) -> Self {
        VarInt(value as i64)
    }
}

struct LabeledTdf(String, Tdf);

enum Tdf {
    VarInt(VarInt),
    String(String),
    Blob(Vec<u8>),
    Group(bool, Vec<LabeledTdf>),
    List(SubDataType, Vec<Tdf>),
    Map(SubDataType, SubDataType, Vec<Tdf>, Vec<Tdf>),
    Union(u8, Option<Tdf>),
    VarIntList(Vec<VarInt>),
    Pair(VarInt, VarInt),
    Tripple(VarInt, VarInt, VarInt),
    Float(f32),
}

trait Writeable: Send + Sync {
    async fn write<W: AsyncWrite>(&self, o: &mut W) -> io::Result<()>;
}

trait Readable: Send + Sync {
    async fn read<R: AsyncRead>(r: &mut R) -> io::Result<Self>;
}

impl Writeable for VarInt {
    async fn write<W: AsyncWrite>(&self, o: &mut W) -> io::Result<()> {
        let value = self.0;
        if value < 0x40 {
            o.write_u8((value & 0xFF) as u8).await?
        } else {
            let mut curr_byte = ((value & 0x3F) as u8) | 0x80;
            o.write_u8(curr_byte).await?;
            let mut curr_shift = value >> 6;
            while curr_shift >= 0x80 {
                curr_byte = ((curr_shift & 0x7F) | 0x80) as u8;
                curr_shift >>= 7;
                o.write_u8(curr_byte).await?;
            }
            o.write_u8(curr_shift as u8).await?;
        }
        Ok(())
    }
}

impl Readable for VarInt {
    async fn read<R: AsyncRead>(r: &mut R) -> io::Result<VarInt> {
        let first = r.read_u8().await?;
        let mut shift = 6;
        let mut result = (first & 0x3F) as i64;
        if first >= 0x80 {
            let mut byte: u8;
            loop {
                byte = r.read_u8().await?;
                result |= (byte & 0x7F) << shift;
                if byte < 0x80 { break; }
            };
        }
        return Ok(VarInt(result));
    }
}

impl Readable for String {
    async fn read<R: AsyncRead>(r: &mut R) -> io::Result<Self> {
        let length = VarInt::read(r).await?.0 as usize;
        let mut bytes = vec![0u8; length - 1];
        r.read_exact(&mut bytes).await?;
        r.read_u8().await?;
        match String::from_utf8(bytes) {
            Ok(value) => Ok(value),
            Err(_) => Err(io::Error::new(ErrorKind::InvalidData, "Invalid utf8 string"))
        }
    }
}

impl Writeable for String {
    async fn write<W: AsyncWrite>(&self, o: &mut W) -> io::Result<()> {
        let mut value = self.clone();
        let null_char = char::from(0);
        if value.len() < 1 {
            value.push(null_char)
        } else if value.chars().last() != null_char {
            value.push(null_char)
        }
        VarInt::from(self.len()).write(o).await?;
        o.write_all(self.as_bytes()).await?;
        Ok(())
    }
}

impl LabeledTdf {
    fn label_to_tag(label: &String) -> [u8; 3] {
        let mut res = [0u8; 3];
        let buff = label.as_bytes();
        res[0] |= ((buf[0] & 0x40) << 1);
        res[0] |= ((buff[0] & 0x40) << 1);
        res[0] |= ((buff[0] & 0x10) << 2);
        res[0] |= ((buff[0] & 0x0F) << 2);
        res[0] |= ((buff[1] & 0x40) >> 5);
        res[0] |= ((buff[1] & 0x10) >> 4);

        res[1] |= ((buff[1] & 0x0F) << 4);
        res[1] |= ((buff[2] & 0x40) >> 3);
        res[1] |= ((buff[2] & 0x10) >> 2);
        res[1] |= ((buff[2] & 0x0C) >> 2);

        res[2] |= ((buff[2] & 0x03) << 6);
        res[2] |= ((buff[3] & 0x40) >> 1);
        res[2] |= (buff[3] & 0x1F);

        return res;
    }

    fn tag_to_label(tag: u32) -> String {
        let buff: [u8; 4] = tag.to_be_bytes();
        let mut res = [0u8; 4];
        res[0] |= ((buff[0] & 0x80) >> 1);
        res[0] |= ((buff[0] & 0x40) >> 2);
        res[0] |= ((buff[0] & 0x30) >> 2);
        res[0] |= ((buff[0] & 0x0C) >> 2);

        res[1] |= ((buff[0] & 0x02) << 5);
        res[1] |= ((buff[0] & 0x01) << 4);
        res[1] |= ((buff[1] & 0xF0) >> 4);

        res[2] |= ((buff[1] & 0x08) << 3);
        res[2] |= ((buff[1] & 0x04) << 2);
        res[2] |= ((buff[1] & 0x03) << 2);
        res[2] |= ((buff[2] & 0xC0) >> 6);

        res[3] |= ((buff[2] & 0x20) << 1);
        res[3] |= (buff[2] & 0x1F);

        return buff.iter()
            .map(|v| if v == 0 { char::from(0x20) } else { char::from(*v) })
            .collect::<String>();
    }
}

impl Writeable for LabeledTdf {
    async fn write<W: AsyncWrite>(&self, o: &mut W) -> io::Result<()> {
        let tdf_type = match self.1 {
            Tdf::VarInt(_) => TdfType::VarInt,
            Tdf::String(_) => TdfType::String,
            Tdf::Blob(_) => TdfType::Blob,
            Tdf::Group(_, _) => TdfType::Group,
            Tdf::List(_, _) => TdfType::List,
            Tdf::Map(_, _, _, _) => TdfType::Map,
            Tdf::Union(_, _) => TdfType::Union,
            Tdf::VarIntList(_) => TdfType::VarIntList,
            Tdf::Pair(_, _) => TdfType::Pair,
            Tdf::Tripple(_, _, _) => TdfType::Tripple,
            Tdf::Float(_) => TdfType::Float
        };
        let tag = LabeledTdf::label_to_tag(label);
        o.write_u8(tag[0]).await?;
        o.write_u8(tag[1]).await?;
        o.write_u8(tag[2]).await?;
        o.write_u8(tdf_type as u8).await?;
        self.1.write(o).await?;
        Ok(())
    }
}

impl Writeable for Tdf {
    async fn write<W: AsyncWrite>(&self, o: &mut W) -> io::Result<()> {
        match self {
            Tdf::VarInt(value) => value.write(o).await?,
            Tdf::String(value) => value.write(o).await?,
            Tdf::Blob(value)=> o.write_all(value).await?,
            Tdf::Group(start2, values) => {
                if start2 { o.write_u8(2).await?; }
                for value in values {
                    value.write(o).await?;
                }
            }
            Tdf::List(sub_type, values) => {
                o.write_u8(sub_type as u8).await?;
                VarInt::from(values.len()).write(o).await?;
                values.iter()
                    .for_each(|v| v.write(o, false));
            }
            Tdf::Map(key_type, value_type, keys, values) => {
                o.write_u8(key_type as u8).await?;
                o.write_u8(value_type as u8).await?;
                let length = keys.len();
                for i in 0..(length - 1) {
                    let key = keys.get(i).unwrap();
                    let value = values.get(i).unwrap();
                    key.write(o).await?;
                    value.write(o).await?;
                }
            }
            Tdf::Union(data, value) => {
                o.write_u8(*data).await?;
                if data != 0x7F && value.is_some() {
                    value.unwrap().write(o).await?;
                }
            }
            Tdf::VarIntList(values) => {
                VarInt::from(values.len()).write(o).await?;
                for value in values {
                    value.write(o).await?;
                }
            }
            Tdf::Pair(a, b) => {
                a.write(o).await?;
                b.write(o).await?;
            }
            Tdf::Tripple(a, b, c) => {
                a.write(o).await?;
                b.write(o).await?;
                c.write(o).await?;
            }
            Tdf::Float(value) => o.write_f32(*value).await?,
        }
        Ok(())
    }
}

