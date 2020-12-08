//! Proof of concept for creating FFI-safe trait objects in Rust.

#![deny(missing_docs)]

mod external;
mod ffi;
mod file_handle;
mod owned;

pub use ffi::*;
pub use file_handle::FileHandle;
pub use owned::OwnedFileHandle;
