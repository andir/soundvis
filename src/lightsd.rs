use std::net::UdpSocket;
use std::sync::mpsc::{Receiver, channel};
use std::thread::spawn;

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


fn send(target: &str, rx: Receiver<Vec<(f32, f32, f32)>>) {
    let sock = UdpSocket::bind("[::]:12345").unwrap();

    while let Ok(d) = rx.recv() {
        let bytes: Vec<u8> = encode(d);
        sock.send_to(&bytes, target).unwrap();
    }
}


pub fn leds(target: &'static str, sample_rx: Receiver<Vec<f32>>) {
    let (tx, rx) = channel();
    let led_count = 2200;
    spawn(move || send(target, rx));
    while let Ok(d) = sample_rx.recv() {
        // some magic!
        let buf: Vec<(f32, f32, f32)> = d.iter()
            .map(|v| ((v * 180.).abs(), 1.0, *v))
            .map(|(h, s, v)| {
                ((180.0 + h), f32::max(s, 0.1), f32::max(v, 0.1))
            })
            .collect();
        let mut b = vec![];
        while b.len() < led_count {
            b.extend(&buf);
        }
        tx.send(b).unwrap();
    }
}
