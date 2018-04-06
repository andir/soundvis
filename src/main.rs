#![feature(slice_rotate)]
#[macro_use]
extern crate gstreamer as gst;
extern crate gstreamer_app as gst_app;

extern crate failure;
extern crate glib;

extern crate rustfft;
extern crate num;
extern crate apodize;

use glium::Surface;

#[macro_use]
extern crate glium;

use glium::glutin::WindowBuilder;
use glium::glutin;

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

use std::net::UdpSocket;

//use std::error::Error as StdError;

mod decoder;
mod simple_decoder;
mod debug;


use std::convert::AsMut;

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
                //println!("len: {}", samples.len());
                //let sum: f64 = samples
                //    .iter()
                //    .map(|sample| {
                //        let f = f64::from(*sample); // / f64::from(i16::MAX);
                //        f * f
                //    })
                //    .sum();
                //println!("rms: {}", sum);
                tx.send(Vec::from(samples)).unwrap();

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

const SAMPLING_DURATION: u64 = 16; // in milliseconds

fn process_loop(rx: Receiver<Vec<f32>>, tx: Sender<Vec<f32>>) {
    let mut dec = simple_decoder::SimpleDecoder::new_simple();
    let mut samples : Vec<f32> = vec![0.0; dec.sample_count];
    let mut fresh_samples = 0;
    let needed_samples = SAMPLING_DURATION as usize * dec.sample_rate / 1000;
    let mut draw_time = std::time::Instant::now();
    while let Ok(d) = rx.recv() {
        let elapsed = draw_time.elapsed();
        //        samples.splice(bins_pos..bins_pos+self.freqs.len(), self.freqs.iter().map(|v| tmp[*v]));
        let new = usize::min(d.len(), dec.sample_count);
        fresh_samples += new;
        samples.rotate_right(new);
        samples.splice(..new, d.into_iter());
        if elapsed <  std::time::Duration::from_millis(SAMPLING_DURATION) {
            continue 
        }
        if fresh_samples >= needed_samples {
            let s = &samples[..dec.sample_count];
            let mut out = dec.decode(s);
            tx.send(out).unwrap();
            fresh_samples = 0;
            draw_time = std::time::Instant::now();
        }
    }
}

fn main() {

    let (raw_tx, raw_rx) = channel();
    let (spec_tx, spec_rx) = channel();

    let pipeline = create_pipeline(raw_tx).expect("A pipline to be created");
    spawn(move || process_loop(raw_rx, spec_tx));
    spawn(move || visual(spec_rx));
    main_loop(pipeline).expect("Clean end.")

}

fn visual(spec_rx: Receiver<Vec<f32>>) {
    use glium::texture::buffer_texture::BufferTexture;
    use glium::texture::buffer_texture::BufferTextureType;

    let window = WindowBuilder::new()
        .with_title("soundvis".to_string())
        .with_dimensions(1024, 786);
    let mut events_loop = glutin::EventsLoop::new();
    let context = glutin::ContextBuilder::new();
    let display = glium::Display::new(window, context, &events_loop).unwrap();
    let program = glium::Program::from_source(
        &display,
        include_str!("../default.glslv"),
        include_str!("../default.glslf"),
        None).unwrap();

    #[derive(Copy, Clone)]
    struct Vertex {
        position: [f32; 2],
    }

    implement_vertex!(Vertex, position);

    let shape = vec![
          Vertex { position: [ -1.0, -1.0]}, //-1.0, -1.0,  0.5,  0.0, ] },
          Vertex { position: [ -1.0, 1.0]}, // 0.0, -1.0,  1.0,  0.5, ] },
          Vertex { position: [ 1.0, -1.0]}, // 0.0,  1.0,  1.0, -1.0, ] },


          Vertex { position: [ -1.0, 1.0]}, // 0.0, -1.0,  1.0,  0.5, ] },
          Vertex { position: [ 1.0, 1.0]}, // 0.0, -1.0,  1.0,  0.5, ] },
          Vertex { position: [ 1.0, -1.0]}, //-1.0, -1.0,  0.5,  0.0, ] },

    ];

    let vertex_buffer = glium::VertexBuffer::new(&display, &shape).unwrap();
    let mut spec = spec_rx.recv().unwrap();
    let mut global_max = 0.0;
    loop {
        let mut max = spec.iter().cloned().fold(0.0, f32::max);
        if max < 0.0 {
            max = 1.0;
        }
        if global_max < max {
            global_max = max;
        }
        let sspec : Vec<f32> = spec.iter().map(|v| {
            if *v < 0.0 {
                return 0.0;
            }
            let n = v / global_max;
            n
        }).collect();
        //println!("v: {:?} {}", sspec, sspec.len());

        let buf_tex = BufferTexture::new(&display, &sspec, BufferTextureType::Float);
        let buf_tex : BufferTexture<f32> = match buf_tex {
            Ok(t) => t,
            Err(_) => return,
        };
        let mut target = display.draw();
        target.clear_color(0., 0., 0., 0.);
        target.draw(&vertex_buffer,
                        &glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList),
                        &program,
                        &uniform!{
                            tex: &buf_tex,
                        },
                        &Default::default()).unwrap();
        target.finish().unwrap();
        events_loop.poll_events(|event| {
            match event {
                glutin::Event::WindowEvent { event, .. } => match event {
                    glutin::WindowEvent::Closed => return,
                    _ => ()
                },
                _ => (),
            }
        });
        spec = spec_rx.recv().unwrap();
    }
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
