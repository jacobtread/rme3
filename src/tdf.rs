use std::any::Any;
use std::collections::HashMap;
use std::io;

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite};

#[repr(u8)]
enum TdfType {
    VarInt = 0x0,
    String = 0x1,
    Blob = 0x2,
    Struct = 0x3,
    List = 0x4,
    Map = 0x5,
    Union = 0x6,
    VarList = 0x7,
    Pair = 0x8,
    Tripple = 0x9,
    Float = 0xA
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
enum ListType {
    VarInt = 0x0,
    String = 0x1,
    Struct = 0x3,
    Tripple = 0x9,
    Float = 0xA
}

enum Tdf {
    VarInt(String, u32),
    String(String, String),
    Blob(String, Box<u8>),
    Struct(String, Vec<Tdf>),
    List(String, Vec<Tdf>),
    Map(String, Vec<MapKey>, Vec<MapValue>),
}

fn read_var_int<R: AsyncRead>(r: &mut R) -> io::Result<u32> {
    r.read_u8().await?;
    return Ok(0)
}

impl Tdf {
    fn write_head<W: AsyncWrite>(o: &mut W, label: &String, tdf_type: TdfType) {}

    fn write<W: AsyncWrite>(&self, o: &mut W) {
        match self {
            Tdf::VarInt(label, _) => {
                Tdf::write_head(o, label, TdfType::VarInt)
            }
            Tdf::String(label, _) => {

            }
            Tdf::Blob(label, _) => {}
            Tdf::Struct(label, _) => {}
            Tdf::List(label, _) => {}
            Tdf::Map(label, keys, values) => {}
        }
    }
}
