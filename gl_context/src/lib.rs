mod gl_context_builder;
mod gl_context_trait;

use gl_context_builder::GLContextBuilder;
pub use gl_context_trait::*;

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "macos")]
#[macro_use]
extern crate objc;

#[cfg(target_os = "macos")]
pub use macos::GLContext;
#[cfg(target_os = "macos")]
pub use macos::*;
