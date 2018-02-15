#![feature(slice_rotate)]
#[macro_use]
extern crate gstreamer as gst;
extern crate gstreamer_app as gst_app;

extern crate failure;
extern crate glib;

extern crate rustfft;
extern crate num;
extern crate apodize;

#[macro_use]
extern crate failure_derive;

extern crate byte_slice_cast;
use byte_slice_cast::*;

use failure::Error;

use std::i16;
use std::f64;
use std::sync::mpsc::{Sender, Receiver, channel};
use std::thread::spawn;

use std::sync::Mutex;
use gst::Cast;
use gst::BinExt;
use gst::ElementExt;

//use std::error::Error as StdError;

mod decoder;
mod debug;

#[derive(Debug, Fail)]
#[fail(display = "Received error from {}: {} (debug: {:?})", src, error, debug)]
struct ErrorMessage {
    src: String,
    error: String,
    debug: Option<String>,
    #[cause]
    cause: glib::Error,
}

fn create_pipeline(tx: Sender<Vec<f32>>) -> Result<gst::Pipeline, Error> {
    gst::init()?;

    let gs = match gst::parse_launch(
        "pulsesrc !
        appsink name=sink max-buffers=1 emit-signals=True
            caps=audio/x-raw,format=F32LE,channels=1,rate=44100",
    ) {
        Ok(gs) => gs,
        Err(e) => {
            println!("Failed to create pipeline: {:}", e);
            std::process::exit(1);
        }
    };

    let pipeline: gst::Pipeline = gs.dynamic_cast::<gst::Pipeline>().expect(
        "a pipeline to be created by the launch command",
    );

    let appsink = pipeline
        .get_by_name("sink")
        .expect("The sink must exist")
        .dynamic_cast::<gst_app::AppSink>()
        .expect("An AppSink instance");

    let tx = Mutex::new(tx);

    appsink.set_callbacks(
        gst_app::AppSinkCallbacks::new()
            .new_sample(move |appsink| {
                let tx = if let Ok(tx) = tx.lock() {
                    tx
                } else {
                    return gst::FlowReturn::Eos;
                };

                let sample = match appsink.pull_sample() {
                    None => return gst::FlowReturn::Eos,
                    Some(sample) => sample,
                };

                let buffer = if let Some(buffer) = sample.get_buffer() {
                    buffer
                } else {
                    gst_element_error!(
                        appsink,
                        gst::ResourceError::Failed,
                        ("Failed to get buffer from appsink")
                    );

                    return gst::FlowReturn::Error;
                };

                let map = if let Some(map) = buffer.map_readable() {
                    map
                } else {
                    gst_element_error!(
                        appsink,
                        gst::ResourceError::Failed,
                        ("Failed to map buffer readable")
                    );

                    return gst::FlowReturn::Error;
                };

                let samples = if let Ok(samples) = map.as_slice().as_slice_of::<f32>() {
                    samples
                } else {
                    gst_element_error!(
                        appsink,
                        gst::ResourceError::Failed,
                        ("Failed to interprete buffer as S16 PCM")
                    );

                    return gst::FlowReturn::Error;
                };
                // println!("len: {}", samples.len());
                // let sum: f64 = samples
                //     .iter()
                //     .map(|sample| {
                //         let f = f64::from(*sample); // / f64::from(i16::MAX);
                //         f * f
                //     })
                //     .sum();
                // println!("rms: {}", sum);
                tx.send(Vec::from(samples));

                gst::FlowReturn::Ok
            })
            .build(),
    );

    Ok(pipeline)
}

fn main_loop(pipeline: gst::Pipeline) -> Result<(), Error> {
    pipeline.set_state(gst::State::Playing).into_result()?;

    let bus = pipeline.get_bus().expect("Pipeline should have a bus");

    while let Some(msg) = bus.timed_pop(gst::CLOCK_TIME_NONE) {
        use gst::MessageView;
        match msg.view() {
            MessageView::Eos(..) => break,
            MessageView::Error(err) => {
                pipeline.set_state(gst::State::Null).into_result()?;
                // Err(ErrorMessage {
                //     src: err.get_src().map(|s| s.get_path_string()).unwrap_or_else(
                //         || {
                //             String::from("None")
                //         },
                //     ),
                //     error: err.get_error().description().into(),
                //     debug: err.get_debug(),
                //     cause: err.get_error(),
                // })?;
            }
            _ => (),
        }
    }
    pipeline.set_state(gst::State::Null).into_result()?;

    Ok(())
}



fn process_loop(rx: Receiver<Vec<f32>>) {
    let sample_size = 441;
    let mut dec = decoder::Decoder::new(44100.0, 7, 8, 15);
    while let Ok(d) = rx.recv() {
        if d.len() != sample_size {
            println!("sample_size {} is not expected size {}", d.len(), sample_size);
            continue
        }
        let out = dec.to_bins(d);
        // for t in 0..out.len() {
        //     let bins = &out[t];
        //     println!("t: {}", t);
        //     println!("freq:\t amp:");
        //     for &bin in bins {
        //         println!("{} {}", bin.freq, bin.amp);
        //     }
        // }
        // debug::write_gnuplot_data(&format!("test-0.data", t), out, |&d| {
        //     (d.freq, d.amp)
        // });

        // std::process::exit(1);
    }
}

fn main() {

    let (raw_tx, raw_rx) = channel();


    let pipeline = create_pipeline(raw_tx).expect("A pipline to be created");
    spawn(move || process_loop(raw_rx));
    main_loop(pipeline).expect("Clean end.")

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
