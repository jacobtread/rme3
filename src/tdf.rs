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
    VarIntList = 0x0,
    String = 0x1,
    Blob = 0x2,
    Group = 0x3,
    List = 0x4,
    Map = 0x5,
    Union = 0x6,
    VarList = 0x7,
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
    VarInt(u32),
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

enum Tdf {
    VarInt {
        label: String,
        value: i64,
    },
    String {
        label: String,
        value: String,
    },
    Blob {
        label: String,
        value: Box<u8>,
    },
    Group {
        label: String,
        start2: bool,
        values: Vec<Tdf>,
    },
    List {
        label: String,
        sub_type: SubDataType,
        values: Vec<Tdf>,
    },
    Map {
        label: String,
        key_type: SubDataType,
        value_type: SubDataType,
        keys: Vec<MapKey>,
        values: Vec<MapValue>,
    },
    Union {
        label: String,
        state: u8,
        value: Option<Tdf>,
    },
    VarIntList {
        label: String,
        values: Vec<i64>,
    },
    Pair {
        label: String,
        a: i64,
        b: i64,
    },
    Tripple {
        label: String,
        a: i64,
        b: i64,
        c: i64,
    },
    Float {
        label: String,
        value: f32,
    },
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


impl Tdf {
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

    async fn write_head<W: AsyncWrite>(o: &mut W, label: &String, tdf_type: TdfType) -> io::Result<()> {
        let tag = Tdf::label_to_tag(label);
        o.write_u8((tag << 24) & 0xFF).await?;
        o.write_u8((tag << 16) & 0xFF).await?;
        o.write_u8((tag << 8) & 0xFF).await?;
        o.write_u8(tdf_type as u8).await?;
        Ok(())
    }
}

impl Writeable for Tdf {
    async fn write<W: AsyncWrite>(&self, o: &mut W) -> io::Result<()> {
        match self {
            Tdf::VarInt { label, value } => {
                Tdf::write_head(o, label, TdfType::VarIntList)
            }
            Tdf::String { label, .. } => {
                Tdf::write_head(o, label, TdfType::String)
            }
            Tdf::Blob { label, .. } => {
                Tdf::write_head(o, label, TdfType::Blob)
            }
            Tdf::Group { label, .. } => {
                Tdf::write_head(o, label, TdfType::Group)
            }
            Tdf::List { label, .. } => {
                Tdf::write_head(o, label, TdfType::List)
            }
            Tdf::Map { label, .. } => {
                Tdf::write_head(o, label, TdfType::Map)
            }
            Tdf::Union { label, .. } => {
                Tdf::write_head(o, label, TdfType::Union)
            }
            Tdf::VarIntList { label, .. } => {
                Tdf::write_head(o, label, TdfType::VarIntList)
            }
            Tdf::Pair { label, .. } => {
                Tdf::write_head(o, label, TdfType::Pair)
            }
            Tdf::Tripple { label, .. } => {
                Tdf::write_head(o, label, TdfType::Tripple)
            }
            Tdf::Float { label, .. } => {
                Tdf::write_head(o, label, TdfType::Float)
            }
        }
    }
}
