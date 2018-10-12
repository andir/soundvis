//#![feature(slice_rotate)]
extern crate apodize;
extern crate byte_slice_cast;
extern crate byteorder;
extern crate failure;
extern crate threadpool;
extern crate num_cpus;
extern crate glib;
#[macro_use]
extern crate glium;
#[macro_use]
extern crate gstreamer;
extern crate gstreamer_app;
extern crate num;
extern crate rustfft;

use std::collections::HashMap;
use std::sync::mpsc::{Sender, Receiver, channel};
use std::thread::spawn;
use threadpool::ThreadPool;
use std::sync::Arc;
use std::sync::Mutex;


mod beat;
mod debug;
mod gst;
mod lightsd;
mod process;
mod simple_decoder;
mod visual;
mod bench;

fn normalize(input: Vec<f32>, global_max: f32) -> (Vec<f32>, f32) {
    let mut max = input.iter().cloned().fold(0.0, f32::max);
    if max < 0.0 {
        max = 1.0;
    }

    let mut global_max = global_max * 0.99;
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
        smooth_values = d.iter()
            .zip(smooth_values.into_iter().chain(std::iter::repeat(0.0)))
            .map(|(val, max)| if *val > max * 1.05 {
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
    T: Clone + Send + 'static + std::fmt::Debug,
{
    let mut tx_channels: Vec<Sender<T>> = Vec::new();
    let mut rx_channels: Vec<Receiver<T>> = Vec::new();

    for _ in 0..n {
        let (tx, rx) = channel();
        tx_channels.push(tx);
        rx_channels.push(rx);
    }

    spawn(move || while let Ok(d) = rx.recv() {
        tx_channels
            .iter()
            .map(|tx| tx.send(d.clone()).unwrap())
            .count();
    });

    rx_channels
}


fn main() {
    let (raw_tx, raw_rx) = channel();

    // configure our gstreamer pipeline
    let pipeline = gst::create_pipeline(raw_tx).expect("A pipline to be created");
    let (leds_tx, leds_rx) = channel();

    spawn(move || lightsd::leds("172.20.64.232:1337", leds_rx));

    let (out_tx, out_rx) = channel();
    spawn(move || visual::visual(out_rx));
    // spawn a thread that handles all the processing of data and passes processed data onwards
    spawn(move || {

        const sample_rate: usize = 44100;

        // create a thread pool to execute everything on
        let pool = ThreadPool::new(num_cpus::get_physical() - 1);

        // create all the fft processors
        let range = 8..14;
        let range_start = range.start;
        let range_end = range.end;

        let processors: Vec<(usize, Arc<Mutex<process::Processor>>)> = range
            .map(|k| {
                (
                    k,
                    Arc::new(Mutex::new(process::Processor::new(k, sample_rate))),
                )
            })
            .collect();

        // cache the last result of an fft
        // this enables us to to updates even if just one fft reported
        // new values
        let mut fft_cache: HashMap<usize, Vec<f32>> = HashMap::new();

        // let (beat_tx, beat_rx) = channel();

        let mut global_max = 0.0;

        // for each received sample frame
        let mut frame_counter = 0;
 //       let mut now = std::time::Instant::now();
        while let Ok(d) = raw_rx.recv() {
            let (tx, rx) = channel();
//            println!("Elapsed {} {} {}", "recv", now.elapsed().subsec_nanos() / 1000, d.len());
            //let (loop_beat_tx, loop_beat_rx) = channel();
            // pass it to the beat detector as task
            //{
            //    let beat_data = d.clone();
            //    let btx = loop_beat_tx.clone();
            //    let mut beat_detector = Arc::clone(&beat_detector);
            //    pool.execute(move || {
            //        let mut beat_detector = beat_detector.lock().expect("Scheduled beat detector more than once");
            //        let d = beat_detector.analyze(&beat_data);
            //        btx.send(d).expect("Result channel must be open");
            //    });
            //}

            // feed it into our fft processs loop
            //
            processors
                .iter()
                .map(|(k, p)| (*k, Arc::clone(p)))
                .map(|(k, p)| {
                    let d = d.clone();
                    let tx = tx.clone();
                    pool.execute(move || {
                        let mut p = p.lock().expect("Processor scheduled more than once");
                        tx.send((k, p.process(d))).expect(
                            "Result channel must be open",
                        );
                    });
                })
                .last();

            // await all the ffts before continuing
            let mut bins = vec![0.0; 7 * 12];
            let cached_results = rx.into_iter()
                .map(|(k, r)| {

                    let r =
                        r.map(|r| {
                            fft_cache.insert(k, r.clone());
                            r
                        }).unwrap_or_else(
                                || fft_cache.get(&k).unwrap_or(&vec![0.0; 7 * 12]).clone(),
                            );
                    (k, r)
                })
                .take(processors.len());
            /*
            let merged_bins = cached_results.fold(bins, |bins, (k, r)|{
                bins.into_iter().zip(r.into_iter())
                    .map(|(a, b)| a + b / factor).collect()
            });
*/
            // k \in [8, 13] = range
            let no_of_points = bins.len();
            cached_results
                .map(|(k, r)| {
                    debug_assert!(no_of_points == r.len());
                    let to = (range_end + 1 - k) * 12;
                    let from = if k == range_end - 1 {
                        0
                    } else {
                        (range_end - k) * 12
                    };
                    bins.splice(from..to, r.into_iter().skip(from).take(to - from));
                })
                .count();

            let (merged_bins, max) = normalize(bins, global_max);
            global_max = max;

            // check if all the ffts did return results, if not pick the previous result of that
            // fft (if available)
            leds_tx.send(merged_bins.clone()).unwrap();
            out_tx.send(merged_bins).unwrap();
        }
    });

    // this drives all the other tasks since we require new audio samples.
    gst::gst_loop(pipeline).expect("Clean end.")
}

//fn old_main() {
//
//    let (raw_tx, raw_rx) = channel();
//
//    let pipeline = gst::create_pipeline(raw_tx).expect("A pipline to be created");
//
//    // create a cloneing output so we can feed the raw samples into the beat detection
//    lt (raw_rx1, raw_rx2) = cloneing_receiver(raw_rx);
//
//    let (beat_tx, beat_rx) = channel();
//
//    spawn(move || {
//        let mut b = beat::SimpleBeatDetector::new(44100);
//        while let Ok(d) = raw_rx1.recv() {
//            let v: bool = b.analyze(&d);
//            if v {
//                println!("BEAT");
//            }
//            match beat_tx.send(v) {
//                Ok(_) => (),
//                Err(e) => panic!("Failed to send bool: {:?}", e),
//            };
//        }
//    });
//
//    // for each window size create a dedicated receiver via our fanout function
//    let range = 12..15;
//    let receivers = fanout_receiver(raw_rx2, range.len());
//    let start = range.start;
//    let mut processed_chans = Vec::new();
//    receivers
//        .into_iter()
//        .enumerate()
//        .map(|(i, rx)| {
//            let (processed_tx, processed_rx) = channel();
//            //let (smooth_processed_tx, smooth_processed_rx) = channel();
//            spawn(move || process_loop(start + i, rx, processed_tx));
//            //spawn(move || smoothing(processed_rx, smooth_processed_tx));
//            processed_chans.push(processed_rx);
//        })
//        .count();
//    let (out_tx, out_rx) = channel();
//    spawn(move || {
//        let mut global_max = 0.0;
//        loop {
//            let mut bins = vec![0.0; 7 * 12]; // FIXME
//            for (k, chan) in processed_chans.iter().enumerate() {
//                let factor = 1.0; //2f32.powi(k as i32);
//                if let Ok(d) = chan.recv() {
//                    bins = bins.into_iter()
//                        .zip(d.iter())
//                        .map(|(a, b)| a + b / factor)
//                        .collect();
//                }
//            }
//            let (bins, max) = normalize(bins, global_max);
//            global_max = max;
//            out_tx.send(bins).unwrap();
//        }
//    });
//    let (rx1, rx2) = cloneing_receiver(out_rx);
//    spawn(move || visual::visual(rx1, beat_rx));
//    spawn(move || lightsd::leds("172.20.64.232:1337", rx2));
//    gst::gst_loop(pipeline).expect("Clean end.")
//}



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
