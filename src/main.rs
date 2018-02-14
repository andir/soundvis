#[macro_use]
extern crate gstreamer as gst;
extern crate gstreamer_app as gst_app;
extern crate failure;
extern crate glib;
extern crate pvoc;
#[macro_use]
extern crate failure_derive;

extern crate byte_slice_cast;
use byte_slice_cast::*;

use failure::Error;

use std::i16;
use std::f64;
use std::sync::mpsc::{Sender, Receiver, channel};
use std::thread::spawn;

use std::sync::Arc;
use std::sync::Mutex;
use gst::Cast;
use gst::BinExt;
use gst::ElementExt;

//use std::error::Error as StdError;


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

                let sum: f64 = samples
                    .iter()
                    .map(|sample| {
                        let f = f64::from(*sample); // / f64::from(i16::MAX);
                        f * f
                    })
                    .sum();
                tx.send(Vec::from(samples));
                println!("rms: {}", sum);

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


fn process_loop(rx: Receiver<Vec<u8>>) {
    use pvoc::{PhaseVocoder, Bin};
    let pvoc = PhaseVocoder::new(1, 44100.0, 256, 4);
    while let Ok(d) = rx.recv() {
        let mut _output: Vec<Bin> = Vec::new();
        let ds: &[u8] = &d;
        let input: &[&[u8]] = &vec![ds];
        let mut output = vec![&_output[..]][..];
        pvoc.process(&input, &mut output, |channels: usize,
         bins: usize,
         input: &[Vec<Bin>],
         output: &mut [Vec<Bin>]| {
            for i in 0..channels {
                for j in 0..bins {
                    output[i][j] = input[i][j];
                }
            }
        });
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
