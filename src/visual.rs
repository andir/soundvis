use std::sync::mpsc::Receiver;
use glium::Surface;
use glium::glutin::WindowBuilder;
use glium::glutin;
use glium;

pub fn visual(spec_rx: Receiver<Vec<f32>>) {
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
        None,
    ).unwrap();

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
    loop {
        //println!("v: {:?} {}", sspec, sspec.len());
        println!("tex len: {}", spec.len());
        let buf_tex = BufferTexture::new(&display, &spec, BufferTextureType::Float);
        let buf_tex: BufferTexture<f32> = match buf_tex {
            Ok(t) => t,
            Err(_) => return,
        };
        let mut target = display.draw();
        target.clear_color(0., 0., 0., 0.);
        target
            .draw(
                &vertex_buffer,
                &glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList),
                &program,
                &uniform!{
                            tex: &buf_tex,
                        },
                &Default::default(),
            )
            .unwrap();
        target.finish().unwrap();
        events_loop.poll_events(|event| match event {
            glutin::Event::WindowEvent { event, .. } => {
                match event {
                    glutin::WindowEvent::Closed => return,
                    _ => (),
                }
            }
            _ => (),
        });
        spec = spec_rx.recv().unwrap();
    }
}
