//use glow::*;
use kettlewin::*;

fn main() {
    let (mut app, mut event_loop) = initialize();
    let window = app.new_window().build().unwrap();

    // Create a GLContext
    let mut gl_context = GLContext::new().build().unwrap();

    // Assign the GLContext's window.
    gl_context.set_window(Some(&window.id)).unwrap();

    // Glow is a library for accessing GL function calls from a variety of platforms
    // Glow requires a cross platform way to load function pointers,
    // which GLContext provides with get_proc_address.

    #[cfg(target_arch = "wasm32")]
    let gl = glow::Context::from_webgl1_context(gl_context.get_webgl1_context());
    #[cfg(not(target_arch = "wasm32"))]
    let _gl = glow::Context::from_loader_function(|s| gl_context.get_proc_address(s));

    /*
        unsafe {
            gl.clear_color(0.3765, 0.3137, 0.8627, 1.0);
            gl.clear(COLOR_BUFFER_BIT | DEPTH_BUFFER_BIT);
        }
        gl_context.swap_buffers();
    */
    event_loop.run(move |event| match event {
        Event::WindowMoved { .. } => {
            println!("EVENT: {:?}", event);
        }
        Event::KeyDown { key } => match key {
            Key::A => {
                app.set_cursor(Cursor::IBeam);
            }
            Key::B => {
                app.set_cursor(Cursor::Arrow);
            }
            _ => {}
        },
        Event::Draw { .. } => {
            //  gl_context.set_window(Some(&window.id)).unwrap();

            // Make the GLContext current to the thread that this callback runs on.
            //gl_context.make_current();

            // Clear the screen to a lovely shade of blue.
            /*
            unsafe {
                gl.clear_color(0.3765, 0.3137, 0.8627, 1.0);
                gl.clear(COLOR_BUFFER_BIT | DEPTH_BUFFER_BIT);
            }
            // Finally display what we've drawn.
            gl_context.swap_buffers();

            // It is not necessary for this example,
            // but calling request_frame ensures the program redraws continuously.
            // window.request_redraw();

            println!("{}", now.elapsed().as_millis());
            now = std::time::Instant::now();
            */
        }
        _ => {}
    });
}
