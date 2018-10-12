use std::sync::Mutex;
use gstreamer;
use gstreamer_app;
use gstreamer::Cast;
use gstreamer::BinExt;
use gstreamer::ElementExt;
use std::sync::mpsc::Sender;
use failure::Error;
use std;
use byte_slice_cast::*;

pub fn create_pipeline(tx: Sender<Vec<f32>>) -> Result<gstreamer::Pipeline, Error> {
    gstreamer::init()?;

    let gs = match gstreamer::parse_launch(
        "pulsesrc blocksize=3288 !
         appsink name=sink max-buffers=1 emit-signals=True
         caps=audio/x-raw,format=F32LE,channels=1,rate=44100
         ",
    ) {
        Ok(gs) => gs,
        Err(e) => {
            println!("Failed to create pipeline: {:}", e);
            std::process::exit(1);
        }
    };

    let pipeline: gstreamer::Pipeline = gs.dynamic_cast::<gstreamer::Pipeline>().expect(
        "a pipeline to be created by the launch command",
    );

    let appsink = pipeline
        .get_by_name("sink")
        .expect("The sink must exist")
        .dynamic_cast::<gstreamer_app::AppSink>()
        .expect("An AppSink instance");

    let tx = Mutex::new(tx);

        appsink.set_callbacks(
        gstreamer_app::AppSinkCallbacks::new()
            .new_sample(move |appsink| {
                let tx = if let Ok(tx) = tx.lock() {
                    tx
                } else {
                    return gstreamer::FlowReturn::Eos;
                };

                let sample = match appsink.pull_sample() {
                    None => return gstreamer::FlowReturn::Eos,
                    Some(sample) => sample,
                };

                let buffer = if let Some(buffer) = sample.get_buffer() {
                    buffer
                } else {
                    gst_element_error!(
                        appsink,
                        gstreamer::ResourceError::Failed,
                        ("Failed to get buffer from appsink")
                    );

                    return gstreamer::FlowReturn::Error;
                };

                let map = if let Some(map) = buffer.map_readable() {
                    map
                } else {
                    gst_element_error!(
                        appsink,
                        gstreamer::ResourceError::Failed,
                        ("Failed to map buffer readable")
                    );

                    return gstreamer::FlowReturn::Error;
                };

                let samples = if let Ok(samples) = map.as_slice().as_slice_of::<f32>() {
                    samples
                } else {
                    gst_element_error!(
                        appsink,
                        gstreamer::ResourceError::Failed,
                        ("Failed to interprete buffer as S16 PCM")
                    );

                    return gstreamer::FlowReturn::Error;
                };
                //println!("len: {}", samples.len());
                //let sum: f64 = samples
                //    .iter()
                //    .map(|sample| {
                //        let f = f64::from(*sample); // / f64::from(i16::MAX);
                //        f * f
                //    })
                //    .sum();
                tx.send(Vec::from(samples)).unwrap();

                gstreamer::FlowReturn::Ok
            })
            .build(),
    );

    Ok(pipeline)
}

pub fn gst_loop(pipeline: gstreamer::Pipeline) -> Result<(), Error> {
    pipeline.set_state(gstreamer::State::Playing).into_result()?;

    let bus = pipeline.get_bus().expect("Pipeline should have a bus");

    while let Some(msg) = bus.timed_pop(gstreamer::CLOCK_TIME_NONE) {
        use gstreamer::MessageView;
        match msg.view() {
            MessageView::Eos(..) => break,
            MessageView::Error(_) => {
                pipeline.set_state(gstreamer::State::Null).into_result()?;
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
    pipeline.set_state(gstreamer::State::Null).into_result()?;

    Ok(())
}
