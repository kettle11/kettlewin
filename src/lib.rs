mod app_parameters;
mod buttons;
mod events;
mod window_parameters;

pub use app_parameters::*;
pub use window_parameters::*;

pub use buttons::Button;
pub use events::*;

#[cfg(all(target_os = "windows"))]
mod windows;
#[cfg(all(target_os = "windows"))]
pub use windows::*;

#[cfg(all(target_os = "macos"))]
mod macos;
#[cfg(all(target_os = "macos"))]
pub use macos::*;

#[cfg(target_arch = "wasm32")]
mod web;
#[cfg(target_arch = "wasm32")]
pub use web::*;

#[cfg(feature = "opengl_glow")]
pub extern crate glow;

#[cfg(all(target_os = "macos"))]
#[macro_use]
extern crate objc;
