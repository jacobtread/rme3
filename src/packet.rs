use std::io;
use std::io::{Cursor, Read};

use tokio::io::{AsyncReadExt};
use tokio::net::TcpStream;

use crate::tdf::{LabeledTdf, Readable};

#[derive(Debug)]
pub struct Packet {
    component: u16,
    command: u16,
    error: u16,
    qtype: u16,
    id: u16,
    content: Vec<u8>,
}

pub async fn read_packet(r: &mut TcpStream) -> io::Result<Packet> {
    let length = r.read_u16().await? as usize;
    let component = r.read_u16().await?;
    let command = r.read_u16().await?;
    let error = r.read_u16().await?;
    let qtype = r.read_u16().await?;
    let id = r.read_u16().await?;
    let ext_length = if (qtype & 0x10) != 0 { r.read_u16().await? } else { 0u16 };
    let content_length = length + ((ext_length as usize) << 16);
    let mut content = vec![0u8; content_length];
    r.read_exact(&mut content).await?;
    Ok(Packet {
        component,
        command,
        error,
        qtype,
        id,
        content,
    })
}

pub fn read_packet_contents(packet: &Packet) -> io::Result<Vec<LabeledTdf>> {
    let raw_content = packet.content.clone();
    let length = raw_content.len();
    let mut cursor = Cursor::new(raw_content);
    let mut content = Vec::new();
    while cursor.position() < length as u64 {
        content.push(LabeledTdf::read(&mut cursor)?);
    }
    return Ok(content);
}
