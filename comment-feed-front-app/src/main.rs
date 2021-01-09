use std::{
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::Instant,
};

use futures::{self, executor::spawn};
use glium::{
    self,
    draw_parameters::{DrawParameters, Stencil},
    glutin::{event::Event, window::Fullscreen},
    Surface,
};
use glium_glyph::{
    self,
    glyph_brush::{
        rusttype::{Font, Scale},
        Section,
    },
    GlyphBrush,
};
use overlay;
use rand;
use rand::Rng;
use serde_derive::Serialize;

use websocket::{self, futures::Future, ClientBuilder, Message, OwnedMessage};
#[derive(Serialize, Clone)]
struct SetChannelBody {
    action: String,
    channel: String,
    new_channel: String,
}

struct Comment {
    body: String,
    position: (f32, f32),
}

fn main() {
    // 1. The **winit::EventsLoop** for handling events.
    let events_loop = glium::glutin::event_loop::EventLoop::new();

    let monitor = {
        let window = glium::glutin::window::WindowBuilder::new()
            .with_transparent(true)
            .build(&events_loop)
            .unwrap();
        window.primary_monitor()
    };

    // 2. Parameters for building the Window.
    let wb = glium::glutin::window::WindowBuilder::new()
        .with_inner_size(glium::glutin::dpi::LogicalSize::new(1024.0, 768.0))
        .with_transparent(true)
        .with_always_on_top(true)
        .with_decorations(false)
        .with_fullscreen(Some(Fullscreen::Borderless(monitor)))
        .with_resizable(false);

    // 3. Parameters for building the OpenGL context.
    let cb = glium::glutin::ContextBuilder::new()
        .with_vsync(true)
        .with_hardware_acceleration(Some(true));

    // 4. Build the Display with the given window and OpenGL context parameters and register the
    //    window with the events_loop.
    let display = glium::Display::new(wb, cb, &events_loop).unwrap();
    let mut overlay = overlay::BorrowedOverlay::new(false, 255, 255);
    {
        let gl_window = display.gl_window();
        let window = gl_window.window(); // this is just immutable borrow
        overlay.deactivate(window);
    }
    // later I want to use `display`

    let dejavu: &[u8] = include_bytes!("../resource/fonts/DejaVuSans-2.37.ttf");
    let fonts = vec![Font::from_bytes(dejavu).unwrap()];

    let mut glyph_brush = GlyphBrush::new(&display, fonts);

    let (msg_tx, msg_rx) = mpsc::channel();
    web_socket(msg_tx);

    let mut comments = Vec::<Comment>::new();

    let mut time_last_frame = Instant::now();
    //let window_id = display.gl_window().window().id();
    let mut rng = rand::thread_rng();
    let mut i = 0;

    let mut target = display.draw();
    events_loop.run(move |event, event_loop_window_target, control_flow| {
        i += 1;
        if (i & 4) == 0 {
            let screen_dims = display.get_framebuffer_dimensions();

            if let Ok(message) = msg_rx.try_recv() {
                comments.push(Comment {
                    body: message,
                    position: (
                        screen_dims.0 as f32,
                        rng.gen_range(0.0..screen_dims.1 as f32),
                    ),
                });
            }

            let time_current_frame = Instant::now();
            comments.iter_mut().for_each(|comment| {
                comment.position.0 -=
                    100.0 * (time_current_frame - time_last_frame).as_secs_f32() as f32;
                glyph_brush.queue(Section {
                    text: &comment.body,
                    bounds: (screen_dims.0 as f32, screen_dims.1 as f32),
                    color: [1.0, 1.0, 1.0, 1.0],
                    screen_position: comment.position,
                    scale: Scale::uniform(50.0),
                    ..Section::default()
                });
            });
            time_last_frame = Instant::now();

            let mut target = display.draw();
            target.clear_color_and_depth((0.0, 0.0, 0.0, 0.0), 0.0);
            glyph_brush.draw_queued(&display, &mut target);
            target.finish().unwrap();
        }
    });
}

fn web_socket(message_sender: Sender<String>) {
    thread::spawn(move || {
        let mut client = ClientBuilder::new(
            "wss://7ht6ij8i09.execute-api.ap-northeast-1.amazonaws.com/production",
        )
        .unwrap()
        .connect_secure(None)
        .unwrap();

        let mut messages = client.incoming_messages();

        while let Some(Ok(message)) = messages.next() {
            dbg!("rrrrrr");
            match message {
                OwnedMessage::Text(message) => {
                    dbg!(message.clone());
                    message_sender.send(message).unwrap();
                }
                _ => {
                    dbg!(message);
                }
            }
        }
    });
}
