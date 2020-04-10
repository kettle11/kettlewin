use super::apple::*;
use super::window_mac::*;
use crate::{
    Cursor, PlatformApplicationTrait, PlatformEventLoopTrait, WindowId, WindowParameters,
};
use kettlewin_platform_common::*;
use std::cell::RefCell;
use std::ffi::c_void;

pub static INSTANCE_DATA_IVAR_ID: &str = "instance_data";
static WINDOW_CLASS_NAME: &str = "KettlewinWindowClass";
static VIEW_CLASS_NAME: &str = "KettlewinViewClass";
static APPLICATION_CLASS_NAME: &str = "KettlewinApplicationClass";

thread_local!(pub static APPLICATION_DATA: RefCell<Box<ApplicationData>> = RefCell::new(Box::new(ApplicationData::new())));

#[allow(clippy::mut_from_ref)]
pub fn get_window_data(this: &Object) -> &mut InnerWindowData {
    unsafe {
        let data: *mut std::ffi::c_void = *this.get_ivar(INSTANCE_DATA_IVAR_ID);
        &mut *(data as *mut InnerWindowData)
    }
}

// Global singleton data shared by all windows and the application struct.
pub struct ApplicationData {
    // Used to construct a new window
    ns_application: *mut Object,
    pub modifier_flags: u64, // Key modifier flags
    cursor_hidden: bool,
    requested_redraw: Vec<WindowId>,
    pub actually_terminate: bool, // Set when quit is called. Indicates the program should quit.
}

impl ApplicationData {
    pub fn new() -> Self {
        Self {
            ns_application: std::ptr::null_mut(),
            modifier_flags: 0,
            cursor_hidden: false,
            requested_redraw: Vec::new(),
            actually_terminate: false,
        }
    }
}

fn window_delegate_declaration() -> *const objc::runtime::Class {
    let superclass = class!(NSResponder);
    let mut decl = ClassDecl::new(WINDOW_CLASS_NAME, superclass).unwrap();
    super::events_mac::add_window_events_to_decl(&mut decl);
    decl.add_ivar::<*mut c_void>(INSTANCE_DATA_IVAR_ID);
    decl.register()
}

fn view_delegate_declaration() -> *const objc::runtime::Class {
    let superclass = class!(NSView);
    let mut decl = ClassDecl::new(VIEW_CLASS_NAME, superclass).unwrap();
    super::events_mac::add_view_events_to_decl(&mut decl);
    decl.add_ivar::<*mut c_void>(INSTANCE_DATA_IVAR_ID);
    decl.register()
}

fn application_delegate_declaration() -> *const objc::runtime::Class {
    let superclass = class!(NSResponder);
    let mut decl = ClassDecl::new(APPLICATION_CLASS_NAME, superclass).unwrap();
    super::events_mac::add_application_events_to_decl(&mut decl);
    decl.add_ivar::<*mut c_void>(INSTANCE_DATA_IVAR_ID);
    decl.register()
}

fn create_run_loop_source() -> CFRunLoopSourceRef {
    extern "C" fn event_loop_proxy_handler(_: *mut std::ffi::c_void) {}

    unsafe {
        let rl = CFRunLoopGetMain();
        let mut context: CFRunLoopSourceContext = std::mem::zeroed();
        context.perform = Some(event_loop_proxy_handler);
        let source =
            CFRunLoopSourceCreate(std::ptr::null_mut(), CFIndex::max_value() - 1, &mut context);
        CFRunLoopAddSource(rl, source, kCFRunLoopCommonModes);
        CFRunLoopWakeUp(rl);
        source
    }
}

extern "C" fn control_flow_end_handler(
    _: CFRunLoopObserverRef,
    _: CFRunLoopActivity,
    _: *mut std::ffi::c_void,
) {
    // Now process all redraw request events
    APPLICATION_DATA.with(|d| {
        let application_data = d.borrow_mut();

        for window_id in &application_data.requested_redraw {
            unsafe {
                let window_view: *mut Object =
                    msg_send![window_id.raw() as *mut Object, contentView];
                let () = msg_send![window_view, setNeedsDisplay: YES];
            }
        }
    });
}

pub struct PlatformEventLoop {
    ns_application: *mut Object,
}

impl PlatformEventLoopTrait for PlatformEventLoop {
    fn run(&self, callback: Box<dyn FnMut(crate::Event)>) {
        event_receiver::set_callback(callback);

        unsafe {
            let () = msg_send![self.ns_application, run];
        }
    }
}

pub struct PlatformApplication {
    // application_data: Rc<RefCell<ApplicationData>>,
    window_class: *const objc::runtime::Class,
    view_class: *const objc::runtime::Class,
    ns_application: *mut Object,
    _run_loop_custom_event_source: CFRunLoopSourceRef,
}

impl PlatformApplicationTrait for PlatformApplication {
    type EventLoop = PlatformEventLoop;

    fn new() -> Self {
        unsafe {
            let ns_application: *mut Object = msg_send![class!(NSApplication), sharedApplication];

            let () = msg_send![
                ns_application,
                setActivationPolicy:
                    NSApplicationActivationPolicy::NSApplicationActivationPolicyRegular
            ];

            // Setup the application delegate to handle application events.
            let ns_application_delegate_class = application_delegate_declaration();
            let ns_application_delegate: *mut Object =
                msg_send![ns_application_delegate_class, new];

            let () = msg_send![ns_application, setDelegate: ns_application_delegate];

            let run_loop_custom_event_source = self::create_run_loop_source();

            APPLICATION_DATA.with(|d| {
                d.borrow_mut().ns_application = ns_application;
            });

            // We only used this context to pass application_data to the observer
            // The values in this data structure will be copied out.
            let observer_context = CFRunLoopObserverContext {
                copyDescription: std::ptr::null(),
                info: std::ptr::null(),
                release: std::ptr::null(),
                version: 0,
                retain: std::ptr::null(),
            };

            let observer = CFRunLoopObserverCreate(
                std::ptr::null_mut(),
                kCFRunLoopBeforeWaiting,
                YES,                  // Indicates we want this to run repeatedly
                CFIndex::min_value(), // The lower the value, the sooner this will run
                control_flow_end_handler,
                &observer_context as *const CFRunLoopObserverContext,
            );
            CFRunLoopAddObserver(CFRunLoopGetMain(), observer, kCFRunLoopCommonModes);

            Self {
                window_class: window_delegate_declaration(),
                view_class: view_delegate_declaration(),
                ns_application,
                _run_loop_custom_event_source: run_loop_custom_event_source,
            }
        }
    }

    fn event_loop(&mut self) -> Self::EventLoop {
        PlatformEventLoop {
            ns_application: self.ns_application,
        }
    }

    fn set_window_position(&mut self, window_id: WindowId, x: u32, y: u32) {
        unsafe {
            let screen: *const Object = msg_send![window_id.raw() as *mut Object, screen];
            let screen_frame: CGRect = msg_send![screen, frame];

            let backing_scale: CGFloat =
                msg_send![window_id.raw() as *mut Object, backingScaleFactor];
            let () =
                msg_send![
                    window_id.raw() as *mut Object,
                    setFrameTopLeftPoint: NSPoint::new((x as f64) / backing_scale, screen_frame.size.height - (y as f64) / backing_scale)];
        }
    }

    fn set_window_dimensions(&mut self, window_id: WindowId, width: u32, height: u32) {
        unsafe {
            let backing_scale: CGFloat =
                msg_send![window_id.raw() as *mut Object, backingScaleFactor];
            let () =
                msg_send![window_id.raw() as *mut Object, setContentSize: NSSize::new((width as f64) / backing_scale, (height as f64) / backing_scale)];
        }
    }

    fn set_window_title(&mut self, window_id: WindowId, title: &str) {
        unsafe {
            let title = NSString::new(&title);
            let () = msg_send![window_id.raw() as *mut Object, setTitle: title.raw];
        }
    }

    fn minimize_window(&mut self, window_id: WindowId) {
        unsafe {
            let () = msg_send![window_id.raw() as *mut Object, miniaturize: nil];
        }
    }

    fn maximize_window(&mut self, _window_id: WindowId) {
        // Not implemented on Mac
        // There is no analogous behavior?
    }

    fn fullscreen_window(&mut self, window_id: WindowId) {
        unsafe {
            let () = msg_send![window_id.raw() as *mut Object, toggleFullScreen: nil];
        }
    }

    fn restore_window(&mut self, _window_id: WindowId) {
        unimplemented!()
    }

    fn close_window(&mut self, window_id: WindowId) {
        unsafe {
            let () = msg_send![window_id.raw() as *mut Object, close];
        }
    }

    fn redraw_window(&mut self, window_id: WindowId) {
        let in_live_resize: bool =
            unsafe { msg_send![window_id.raw() as *mut Object, inLiveResize] };

        // If resizing the window don't send a redraw request as it will get one
        // anyways
        if !in_live_resize {
            APPLICATION_DATA.with(|d| {
                let mut application_data = d.borrow_mut();
                application_data
                    .requested_redraw
                    .push(window_id);
            });
        }
    }
    fn set_mouse_position(&mut self, _x: u32, _y: u32) {
        // Need to account for backing scale here!

        /*
        CGWarpMouseCursorPosition(CGPoint {
            x: x as f64,
            y: y as f64,
        });
        */

        unimplemented!()
    }

    // https://developer.apple.com/documentation/appkit/nscursor?language=objc
    fn set_cursor(&mut self, cursor: Cursor) {
        let ns_cursor = class!(NSCursor);
        let cursor: *mut Object = unsafe {
            match cursor {
                Cursor::Arrow => msg_send![ns_cursor, arrowCursor],
                Cursor::IBeam => msg_send![ns_cursor, IBeamCursor],
                Cursor::PointingHand => msg_send![ns_cursor, pointingHandCursor],
                Cursor::OpenHand => msg_send![ns_cursor, openHandCursor],
                Cursor::ClosedHand => msg_send![ns_cursor, closedHandCursor],
            }
        };
        let () = unsafe { msg_send![cursor, set] };
    }

    // Calls to NSCursor hide and unhide must be balanced.
    // So here we track their state and only call hide if the cursor is not already hidden.
    //https://developer.apple.com/documentation/appkit/nscursor?language=objc
    fn hide_cursor(&mut self) {
        APPLICATION_DATA.with(|d| {
            let mut application_data = d.borrow_mut();

            if !application_data.cursor_hidden {
                let ns_cursor = class!(NSCursor);
                let () = unsafe { msg_send![ns_cursor, hide] };
                application_data.cursor_hidden = true;
            }
        });
    }

    //https://developer.apple.com/documentation/appkit/nscursor?language=objc
    fn show_cursor(&mut self) {
        APPLICATION_DATA.with(|d| {
            let mut application_data = d.borrow_mut();

            if application_data.cursor_hidden {
                let ns_cursor = class!(NSCursor);
                let () = unsafe { msg_send![ns_cursor, unhide] };
                application_data.cursor_hidden = false;
            }
        });
    }

    fn new_window(&mut self, window_parameters: &WindowParameters) -> WindowId {
        let result =
            super::window_mac::build(window_parameters, self.window_class, self.view_class);
        result.unwrap()
    }

    fn quit(&self) {
        unsafe {
            let ns_application = {
                // This thread local cannot be accessed if the program is already terminating.
                APPLICATION_DATA.try_with(|d| {
                    d.borrow_mut().actually_terminate = true;
                    d.borrow_mut().ns_application
                })
            };
            
            if let Ok(ns_application) = ns_application {
                let () = msg_send![ns_application, terminate: nil];
            } 
        }
    }

    fn raw_window_handle(&self, window_id: WindowId) -> RawWindowHandle {
        let ns_window = unsafe {window_id.raw() };
        let ns_view: *mut c_void = unsafe { msg_send![ns_window as *mut Object, contentView] };
        raw_window_handle::RawWindowHandle::MacOS (raw_window_handle::macos::MacOSHandle{
           ns_window,
           ns_view,
           ..raw_window_handle::macos::MacOSHandle::empty()
       })
    }
}

