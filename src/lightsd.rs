use std::net::UdpSocket;
use std::sync::mpsc::Receiver;

use byteorder::{LittleEndian, WriteBytesExt};

fn encode(data: Vec<(f32, f32, f32)>) -> Vec<u8> {
    let mut wrt = vec![];

    for d in data.iter() {
        wrt.write_f32::<LittleEndian>(d.0).unwrap();
        wrt.write_f32::<LittleEndian>(d.1).unwrap();
        wrt.write_f32::<LittleEndian>(d.2).unwrap();
    }
    wrt
}


pub fn send(target: &str, rx: Receiver<Vec<(f32, f32, f32)>>) {
    let sock = UdpSocket::bind("[::]:12345").unwrap();

    while let Ok(d) = rx.recv() {
        let bytes : Vec<u8> = encode(d);
        sock.send_to(&bytes, target).unwrap();
    }
}
