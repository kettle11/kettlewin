extern crate kettlewin;
use kettlewin::glow::*;
use kettlewin::*;

fn main() {
    // Create a new window manager with default settings.
    let mut app = App::new(&AppParameters::default()).unwrap();
    let gl = app.gl_context();
    let _window = app
        .new_window(&WindowParameters {
            title: Some("Hello"),
            ..Default::default()
        })
        .unwrap();

    // Run forever
    let mut color = 0.0;

    app.run(move |event, app| match event {
        Event::ResizedWindow { width, height } => {
            println!("Resized: {:?}, {:?}", width, height);
        }
        Event::Draw => {
            unsafe {
                gl.clear_color(0.0, color, 1.0, 1.0);
                gl.clear(COLOR_BUFFER_BIT | DEPTH_BUFFER_BIT);
            }
            color += 0.01;
            // When we're done rendering swap the window buffers to display to the screen.
            app.swap_buffers();

            // app.request_frame();
        }
        _ => {}
    });
}
