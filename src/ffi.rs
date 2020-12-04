use std::{
    ffi::CStr,
    fs::File,
    os::raw::{c_char, c_int},
    ptr,
};

use crate::FileHandle;

/// Create a new [`FileHandle`] which throws away all data written to it.
#[no_mangle]
pub unsafe extern "C" fn new_null_file_handle() -> *mut FileHandle {
    FileHandle::for_writer(std::io::sink())
}

/// Create a new [`FileHandle`] which writes directly to stdout.
#[no_mangle]
pub unsafe extern "C" fn new_stdout_file_handle() -> *mut FileHandle {
    FileHandle::for_writer(std::io::stdout())
}

/// Create a new [`FileHandle`] which will write to a file on disk.
#[no_mangle]
pub unsafe extern "C" fn new_file_handle_from_path(path: *const c_char) -> *mut FileHandle {
    let path = match CStr::from_ptr(path).to_str().ok() {
        Some(p) => p,
        None => return ptr::null_mut(),
    };

    let f = match File::create(path) {
        Ok(f) => f,
        Err(_) => return ptr::null_mut(),
    };

    FileHandle::for_writer(f)
}

/// Free the [`FileHandle`], calling any destructors and cleaning up any
/// resources being used.
#[no_mangle]
pub unsafe extern "C" fn file_handle_destroy(handle: *mut FileHandle) {
    let destructor = (*handle).destroy;
    destructor(handle);
}

/// Write some data to the file handle, returning the number of bytes written.
#[no_mangle]
pub unsafe extern "C" fn file_handle_write(
    handle: *mut FileHandle,
    data: *const c_char,
    len: c_int,
) -> c_int {
    let write = (*handle).write;
    let data = std::slice::from_raw_parts(data as *const u8, len as usize);

    match write(handle, data) {
        Ok(bytes_written) => bytes_written as c_int,
        Err(e) => e.raw_os_error().unwrap_or(-1),
    }
}

/// Flush this output stream, ensuring that all intermediately buffered contents
/// reach their destination.
#[no_mangle]
pub unsafe extern "C" fn file_handle_flush(handle: *mut FileHandle) -> c_int {
    let flush = (*handle).flush;

    match flush(handle) {
        Ok(_) => 0,
        Err(e) => e.raw_os_error().unwrap_or(-1),
    }
}
