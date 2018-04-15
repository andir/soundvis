#![feature(slice_rotate)]
#[macro_use]
extern crate gstreamer;
extern crate gstreamer_app;

extern crate failure;
extern crate glib;

extern crate rustfft;
extern crate num;
extern crate apodize;

extern crate byteorder;

#[macro_use]
extern crate glium;

extern crate byte_slice_cast;


use std::sync::mpsc::{Sender, Receiver, channel};
use std::thread::spawn;


mod lightsd;
//mod decoder;
mod simple_decoder;
mod debug;
mod visual;
mod gst;



fn normalize(input: Vec<f32>, global_max: f32) -> (Vec<f32>, f32) {
let mut max = input.iter().cloned().fold(0.0, f32::max);
if max < 0.0 {
    max = 1.0;
}

let mut global_max =global_max * 0.99;
if global_max < max {
    global_max = max;
}
let out: Vec<f32> = input.iter()
    .map(|v| v / global_max)
   // .map(|v| v.log(10.0) / 2.5 + 1.0)
   // .map(|v| if v < 0.0 { 0.0 } else { v })
    .collect();

    (out, global_max)

}

const SAMPLING_DURATION: u64 = 16; // in milliseconds
fn process_loop(n: usize, rx: Receiver<Vec<f32>>, tx: Sender<Vec<f32>>) {
    let mut dec = simple_decoder::SimpleDecoder::new(2usize.pow(n as u32), 44100);
    let mut samples: Vec<f32> = vec![0.0; dec.sample_count];
    let mut fresh_samples = 0;
    let needed_samples = SAMPLING_DURATION as usize * dec.sample_rate / 1000;
    let mut draw_time = std::time::Instant::now();
    while let Ok(d) = rx.recv() {
        let elapsed = draw_time.elapsed();
        let new = usize::min(d.len(), dec.sample_count);
        fresh_samples += d.len();
        samples.rotate_right(new);
        samples.splice(..new, d.into_iter().take(new));
        if elapsed < std::time::Duration::from_millis(SAMPLING_DURATION) {
            continue;
        }
        if fresh_samples >= needed_samples {
            let s = &samples[..dec.sample_count];
            let out = dec.decode(s);
            tx.send(out).unwrap();
            fresh_samples = 0;
            draw_time = std::time::Instant::now();
        }
    }
}

fn smoothing(rx: Receiver<Vec<f32>>, tx: Sender<Vec<f32>>) {
    let mut draw_time = std::time::Instant::now();
    let mut smooth_values = vec![];
    while let Ok(d) = rx.recv() {
        let elapsed = draw_time.elapsed();
        smooth_values = d.iter().zip(smooth_values.into_iter().chain(std::iter::repeat(0.0)))
            .map(|(val, max)|
                 if *val > max * 1.05 {
                *val
            } else if *val < 0.75 {
                0.09_f32.powf(elapsed.subsec_nanos() as f32 / 1000000000.0) * max
//                0.9_f32.powf(elapsed.subsec_nanos() as f32 * 1000000000.) * max
                } else {
               max
            })
            .collect();
        tx.send(smooth_values.clone()).unwrap();
        draw_time = std::time::Instant::now();
    }
}


fn cloneing_receiver<T>(rx: Receiver<T>) -> (Receiver<T>, Receiver<T>)
where
    T: Clone + Send + 'static,
{
    let (tx1, rx1) = channel();
    let (tx2, rx2) = channel();

    spawn(move || while let Ok(d) = rx.recv() {
        tx1.send(d.clone()).unwrap();
        tx2.send(d).unwrap();
    });

    (rx1, rx2)
}

fn fanout_receiver<T>(rx: Receiver<T>, n: usize) -> Vec<Receiver<T>>
where
    T: Clone + Send + 'static + std::fmt::Debug
{
    let mut tx_channels : Vec<Sender<T>> = Vec::new();
    let mut rx_channels : Vec<Receiver<T>> = Vec::new();

    for _ in 0..n {
        let (tx, rx) = channel();
        tx_channels.push(tx);
        rx_channels.push(rx);
    }

    spawn(move || while let Ok(d) = rx.recv() {
        tx_channels.iter().map(|tx| tx.send(d.clone()).unwrap()).count();
    });

    rx_channels
}

fn main() {

    let (raw_tx, raw_rx) = channel();

    let pipeline = gst::create_pipeline(raw_tx).expect("A pipline to be created");

    //let (spec_rx1, spec_rx2) = cloneing_receiver(smooth_processed_rx);
    let range = 12..15;
    let receivers = fanout_receiver(raw_rx, range.len());
    let start = range.start;
    let mut processed_chans = Vec::new();
    receivers.into_iter().enumerate().map(|(i, rx)| {
        let (processed_tx, processed_rx) = channel();
        //let (smooth_processed_tx, smooth_processed_rx) = channel();
        spawn(move || process_loop(start + i, rx, processed_tx));
        //spawn(move || smoothing(processed_rx, smooth_processed_tx));
        processed_chans.push(processed_rx);
    }).count();
    let (out_tx, out_rx) = channel();
    spawn( move || {
        let mut global_max = 0.0;
        loop {
            let mut bins = vec![0.0; 7 * 12]; // FIXME
            for (k, chan) in processed_chans.iter().enumerate() {
                let factor = 1.0; //2f32.powi(k as i32);
                if let Ok(d) = chan.recv() {
                    bins = bins.into_iter().zip(d.iter()).map(|(a, b)| a+b/factor).collect();
                }
            }
            let (bins, max) = normalize(bins, global_max);
            global_max = max;
            out_tx.send(bins).unwrap();
        }
    });
    let (rx1, rx2) = cloneing_receiver(out_rx);
    spawn(move || visual::visual(rx1));
    spawn(move || lightsd::leds("172.20.64.232:1337",rx2));
    gst::gst_loop(pipeline).expect("Clean end.")
}



#[cfg(test)]
mod tests {
    use super::create_pipeline;
    use gst::BinExt;

    #[test]
    fn test_create_pipeline() {
        let pipeline = {
            let p = create_pipeline();
            assert!(p.is_ok());
            p.unwrap()
        };
        assert!(pipeline.get_by_name("sink").is_some());
    }
}
