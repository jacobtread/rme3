use std::any::Any;
use std::collections::HashMap;
use std::fs::read_to_string;
use std::io;
use std::io::{ErrorKind, Read, Write};
use std::ops::Add;
use std::string::FromUtf8Error;

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

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


enum MapKey {
    VarInt(u32),
    String(String),
}

enum MapValue {
    VarInt(VarInt),
    String(String),
    Struct(Vec<Tdf>),
    Float(f32),
}

#[derive(Clone)]
enum SubDataType {
    VarInt,
    String,
    Struct,
    Tripple,
    Float,
}

impl SubDataType {
    fn value(&self) -> u8 {
        match self {
            SubDataType::VarInt => 0x0,
            SubDataType::String => 0x1,
            SubDataType::Struct => 0x3,
            SubDataType::Tripple => 0x9,
            SubDataType::Float => 0xA,
        }
    }
}

#[derive( Clone)]
pub struct VarInt(pub i64);

impl From<usize> for VarInt {
    fn from(value: usize) -> Self {
        VarInt(value as i64)
    }
}

#[derive(Clone)]
struct LabeledTdf(String, Tdf);

#[derive(Clone)]
enum Tdf {
    VarInt(VarInt),
    String(String),
    Blob(Vec<u8>),
    Group(bool, Vec<LabeledTdf>),
    List(SubDataType, Vec<Tdf>),
    Map(SubDataType, SubDataType, Vec<Tdf>, Vec<Tdf>),
    Union(u8, Option<Box<Tdf>>),
    VarIntList(Vec<VarInt>),
    Pair(VarInt, VarInt),
    Tripple(VarInt, VarInt, VarInt),
    Float(f32),
}

impl Tdf {
    fn read<R: Read>(r: &mut R, tdf_type: TdfType) -> io::Result<Self> {
        match tdf_type {
            TdfType::VarInt => {}
            TdfType::String => {}
            TdfType::Blob => {}
            TdfType::Group => {}
            TdfType::List => {}
            TdfType::Map => {}
            TdfType::Union => {}
            TdfType::VarIntList => {}
            TdfType::Pair => {}
            TdfType::Tripple => {}
            TdfType::Float => {}
        }
        Ok(Tdf::String(String::from("")))
    }
}


trait Writeable: Send + Sync {
    fn write<W: Write>(&self, o: &mut W) -> io::Result<()>;
}

trait Readable: Send + Sync {
    fn read<R: Read>(r: &mut R) -> io::Result<Self> where Self: Sized;
}

impl Writeable for VarInt {
    fn write<W: Write>(&self, o: &mut W) -> io::Result<()> {
        let value = self.0;
        if value < 0x40 {
            o.write_u8((value & 0xFF) as u8)?
        } else {
            let mut curr_byte = ((value & 0x3F) as u8) | 0x80;
            o.write_u8(curr_byte)?;
            let mut curr_shift = value >> 6;
            while curr_shift >= 0x80 {
                curr_byte = ((curr_shift & 0x7F) | 0x80) as u8;
                curr_shift >>= 7;
                o.write_u8(curr_byte)?;
            }
            o.write_u8(curr_shift as u8)?;
        }
        Ok(())
    }
}

impl Readable for VarInt {
    fn read<R: Read>(r: &mut R) -> io::Result<VarInt> {
        let first = r.read_u8()?;
        let mut shift = 6;
        let mut result = (first & 0x3F) as i64;
        if first >= 0x80 {
            let mut byte: u8;
            loop {
                byte = r.read_u8()?;
                result |= ((byte & 0x7F) as i64) << shift;
                if byte < 0x80 { break; }
            };
        }
        return Ok(VarInt(result));
    }
}

impl Readable for String {
    fn read<R: Read>(r: &mut R) -> io::Result<Self> {
        let length = VarInt::read(r)?.0 as usize;
        let mut bytes = vec![0u8; length - 1];
        r.read_exact(&mut bytes)?;
        r.read_u8()?;
        match String::from_utf8(bytes) {
            Ok(value) => Ok(value),
            Err(_) => Err(io::Error::new(ErrorKind::InvalidData, "Invalid utf8 string"))
        }
    }
}

impl Writeable for String {
    fn write<W: Write>(&self, o: &mut W) -> io::Result<()> {
        let mut value = self.clone();
        let null_char = char::from(0);
        if value.len() < 1 {
            value.push(null_char)
        } else if value.chars().last().unwrap() != null_char {
            value.push(null_char)
        }
        VarInt::from(self.len()).write(o)?;
        o.write_all(self.as_bytes())?;
        Ok(())
    }
}

impl LabeledTdf {
    fn label_to_tag(label: &String) -> [u8; 3] {
        let mut res = [0u8; 3];
        let buff = label.as_bytes();
        res[0] |= ((buff[0] & 0x40) << 1);
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
        res[0] |= (buff[0] & 0x80) >> 1;
        res[0] |= (buff[0] & 0x40) >> 2;
        res[0] |= (buff[0] & 0x30) >> 2;
        res[0] |= (buff[0] & 0x0C) >> 2;

        res[1] |= (buff[0] & 0x02) << 5;
        res[1] |= (buff[0] & 0x01) << 4;
        res[1] |= (buff[1] & 0xF0) >> 4;

        res[2] |= (buff[1] & 0x08) << 3;
        res[2] |= (buff[1] & 0x04) << 2;
        res[2] |= (buff[1] & 0x03) << 2;
        res[2] |= (buff[2] & 0xC0) >> 6;

        res[3] |= (buff[2] & 0x20) << 1;
        res[3] |= (buff[2] & 0x1F);

        return buff.iter()
            .map(|v| if *v == 0 { char::from(0x20) } else { char::from(*v) })
            .collect::<String>();
    }
}

impl Writeable for LabeledTdf {
    fn write<W: Write>(&self, o: &mut W) -> io::Result<()> {
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
        let tag = LabeledTdf::label_to_tag(&self.0);
        o.write_u8(tag[0])?;
        o.write_u8(tag[1])?;
        o.write_u8(tag[2])?;
        o.write_u8(tdf_type as u8)?;
        self.1.write(o)?;
        Ok(())
    }
}


impl Writeable for Tdf {
    fn write<W: Write>(&self, o: &mut W) -> io::Result<()> {
        match self {
            Tdf::VarInt(value) => value.write(o)?,
            Tdf::String(value) => value.write(o)?,
            Tdf::Blob(value) => o.write_all(value)?,
            Tdf::Group(start2, values) => {
                if *start2 { o.write_u8(2)?; }
                for value in values {
                    value.write(o)?;
                }
            }
            Tdf::List(sub_type, values) => {
                o.write_u8(sub_type.value())?;
                VarInt::from(values.len()).write(o)?;
                for value in values {
                    value.write(o)?;
                }
            }
            Tdf::Map(key_type, value_type, keys, values) => {
                o.write_u8(key_type.value())?;
                o.write_u8(value_type.value())?;
                let length = keys.len();
                for i in 0..(length - 1) {
                    let key = keys.get(i).unwrap();
                    let value = values.get(i).unwrap();
                    key.write(o)?;
                    value.write(o)?;
                }
            }
            Tdf::Union(data, value) => {
                o.write_u8(*data)?;
                if *data != 0x7F {
                    let v = value.as_ref().unwrap().as_ref();
                    v.write(o)?;
                }
            }
            Tdf::VarIntList(values) => {
                VarInt::from(values.len()).write(o)?;
                for value in values {
                    value.write(o)?;
                }
            }
            Tdf::Pair(a, b) => {
                a.write(o)?;
                b.write(o)?;
            }
            Tdf::Tripple(a, b, c) => {
                a.write(o)?;
                b.write(o)?;
                c.write(o)?;
            }
            Tdf::Float(value) => o.write_f32::<BigEndian>(*value)?,
        }
        Ok(())
    }
}
