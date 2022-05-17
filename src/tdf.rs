use std::io;
use std::io::{ErrorKind, Read, Seek, SeekFrom, Write};

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

#[derive(Clone, Debug)]
pub enum TdfType {
    VarInt,
    String,
    Blob,
    Group,
    List,
    Map,
    Union,
    VarIntList,
    Pair,
    Tripple,
    Float,
    Unknown(u8),
}

impl TdfType {
    fn value(&self) -> u8 {
        match self {
            TdfType::VarInt => 0x0,
            TdfType::String => 0x1,
            TdfType::Blob => 0x2,
            TdfType::Group => 0x3,
            TdfType::List => 0x4,
            TdfType::Map => 0x5,
            TdfType::Union => 0x6,
            TdfType::VarIntList => 0x7,
            TdfType::Pair => 0x8,
            TdfType::Tripple => 0x9,
            TdfType::Float => 0xA,
            TdfType::Unknown(value) => *value
        }
    }
}

impl From<u8> for TdfType {
    fn from(value: u8) -> Self {
        match value {
            0x0 => TdfType::VarInt,
            0x1 => TdfType::String,
            0x2 => TdfType::Blob,
            0x3 => TdfType::Group,
            0x4 => TdfType::List,
            0x5 => TdfType::Map,
            0x6 => TdfType::Union,
            0x7 => TdfType::VarIntList,
            0x8 => TdfType::Pair,
            0x9 => TdfType::Tripple,
            0xA => TdfType::Float,
            value => TdfType::Unknown(value),
        }
    }
}

#[derive(Clone, Debug)]
pub struct VarInt(pub i64);

impl From<usize> for VarInt {
    fn from(value: usize) -> Self {
        VarInt(value as i64)
    }
}

#[derive(Clone, Debug)]
pub struct LabeledTdf(pub String, pub TdfType, pub Tdf);

#[derive(Clone, Debug)]
pub enum Tdf {
    VarInt(VarInt),
    String(String),
    Blob(Vec<u8>),
    Group(bool, Vec<LabeledTdf>),
    List(TdfType, Vec<Tdf>),
    Map(TdfType, TdfType, Vec<Tdf>, Vec<Tdf>),
    Union(u8, Option<Box<LabeledTdf>>),
    VarIntList(Vec<VarInt>),
    Pair(VarInt, VarInt),
    Tripple(VarInt, VarInt, VarInt),
    Float(f32),
    Unknown,
}


pub trait Writeable: Send + Sync {
    fn write<W: Write>(&self, o: &mut W) -> io::Result<()>;
}

pub trait Readable: Send + Sync {
    fn read<R: Read + Seek>(r: &mut R) -> io::Result<Self> where Self: Sized;
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
    fn read<R: Read + Seek>(r: &mut R) -> io::Result<VarInt> {
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

impl Readable for String {
    fn read<R: Read + Seek>(r: &mut R) -> io::Result<Self> {
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

impl LabeledTdf {
    fn label_to_tag(label: &String) -> [u8; 3] {
        let mut res = [0u8; 3];
        let buff = label.as_bytes();
        res[0] |= (buff[0] & 0x40) << 1;
        res[0] |= (buff[0] & 0x40) << 1;
        res[0] |= (buff[0] & 0x10) << 2;
        res[0] |= (buff[0] & 0x0F) << 2;
        res[0] |= (buff[1] & 0x40) >> 5;
        res[0] |= (buff[1] & 0x10) >> 4;

        res[1] |= (buff[1] & 0x0F) << 4;
        res[1] |= (buff[2] & 0x40) >> 3;
        res[1] |= (buff[2] & 0x10) >> 2;
        res[1] |= (buff[2] & 0x0C) >> 2;

        res[2] |= (buff[2] & 0x03) << 6;
        res[2] |= (buff[3] & 0x40) >> 1;
        res[2] |= buff[3] & 0x1F;

        return res;
    }

    fn tag_to_label(tag: u32) -> String {
        let mut buff: [u8; 4] = tag.to_be_bytes();
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
        res[3] |= buff[2] & 0x1F;

        return res.iter()
            .filter_map(|v| if *v == 0 { None } else { Some(char::from(*v)) })
            .collect::<String>();
    }
}

impl Writeable for LabeledTdf {
    fn write<W: Write>(&self, o: &mut W) -> io::Result<()> {
        let tag = LabeledTdf::label_to_tag(&self.0);
        o.write_u8(tag[0])?;
        o.write_u8(tag[1])?;
        o.write_u8(tag[2])?;
        o.write_u8(self.1.value())?;
        self.2.write(o)?;
        Ok(())
    }
}

impl Readable for LabeledTdf {
    fn read<R: Read + Seek>(r: &mut R) -> io::Result<Self> where Self: Sized {
        let head = r.read_u32::<BigEndian>()?;
        let tag = head & 0xFFFFFF00;
        let label = LabeledTdf::tag_to_label(tag);
        let tdf_type = TdfType::from((head & 0xFF) as u8);
        let tdf = Tdf::read(r, &tdf_type)?;
        Ok(LabeledTdf(label, tdf_type, tdf))
    }
}

impl Writeable for Tdf {
    fn write<W: Write>(&self, o: &mut W) -> io::Result<()> {
        match self {
            Tdf::VarInt(value) => value.write(o)?,
            Tdf::String(value) => value.write(o)?,
            Tdf::Blob(value) => {
                VarInt::from(value.len()).write(o)?;
                o.write_all(value)?;
            }
            Tdf::Group(start2, values) => {
                if *start2 { o.write_u8(2)?; }
                for value in values {
                    value.write(o)?;
                }
                o.write_u8(0)?;
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
            Tdf::Unknown => {}
        }
        Ok(())
    }
}

type TdfResult<R> = Result<R, TdfError>;

enum TdfError {
    MissingLabel,
    NotGroup,
    InvalidType,
}

impl Tdf {
    fn read<R: Read + Seek>(r: &mut R, tdf_type: &TdfType) -> io::Result<Self> {
        Ok(match tdf_type {
            TdfType::VarInt => Tdf::VarInt(VarInt::read(r)?),
            TdfType::String => Tdf::String(String::read(r)?),
            TdfType::Blob => {
                let size = VarInt::read(r)?.0 as usize;
                let mut bytes = vec![0u8; size];
                r.read_exact(&mut bytes)?;
                Tdf::Blob(bytes)
            }
            TdfType::Group => {
                let mut first_two = false;
                let mut values: Vec<LabeledTdf> = Vec::new();
                'group: loop {
                    let first = r.read_u8()?;
                    if first == 0 {
                        break 'group;
                    } else if first == 2 {
                        first_two = true;
                    } else {
                        r.seek(SeekFrom::Current(-1))?;
                    }
                    values.push(LabeledTdf::read(r)?);
                };
                Tdf::Group(first_two, values)
            }
            TdfType::List => {
                let sub_type = TdfType::from(r.read_u8()?);
                let length = VarInt::read(r)?.0 as usize;
                let mut values = Vec::with_capacity(length);
                for _ in 0..(length - 1) {
                    values.push(Tdf::read(r, &sub_type)?);
                }
                Tdf::List(sub_type, values)
            }
            TdfType::Map => {
                let key_type = TdfType::from(r.read_u8()?);
                let value_type = TdfType::from(r.read_u8()?);
                let length = VarInt::read(r)?.0 as usize;
                let mut keys = Vec::with_capacity(length);
                let mut values = Vec::with_capacity(length);
                for _ in 0..(length - 1) {
                    keys.push(Tdf::read(r, &key_type)?);
                    values.push(Tdf::read(r, &value_type)?);
                }
                Tdf::Map(key_type, value_type, keys, values)
            }
            TdfType::Union => {
                let data = r.read_u8()?;
                let value = if data != 0x7F {
                    Some(Box::new(LabeledTdf::read(r)?))
                } else {
                    None
                };
                Tdf::Union(data, value)
            }
            TdfType::VarIntList => {
                let length = VarInt::read(r)?.0 as usize;
                let mut values = Vec::with_capacity(length);
                for _ in 0..(length - 1) {
                    values.push(VarInt::read(r)?);
                }
                Tdf::VarIntList(values)
            }
            TdfType::Pair => {
                let a = VarInt::read(r)?;
                let b = VarInt::read(r)?;
                Tdf::Pair(a, b)
            }
            TdfType::Tripple => {
                let a = VarInt::read(r)?;
                let b = VarInt::read(r)?;
                let c = VarInt::read(r)?;
                Tdf::Tripple(a, b, c)
            }
            TdfType::Float => {
                let value = r.read_f32::<BigEndian>()?;
                Tdf::Float(value)
            }
            TdfType::Unknown(_) => Tdf::Unknown
        })
    }

    fn get_text(&self, label: &str) -> TdfResult<String> {
        if let Tdf::Group(_, values) = self {
            for value in values {
                if value.0 == label {
                    if let Tdf::String(text) = &value.1 {
                        Ok(text.clone())
                    } else {
                        Err(TdfError::InvalidType)
                    }
                }
            }
            Err(TdfError::MissingLabel)
        } else {
            Err(TdfError::NotGroup)
        }
    }
}
