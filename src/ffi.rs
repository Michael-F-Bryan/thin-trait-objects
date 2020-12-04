use std::{
    ffi::CStr,
    fs::File,
    os::raw::{c_char, c_int},
    ptr,
};

use crate::FileHandle;

#[no_mangle]
pub unsafe extern "C" fn new_null_file_handle() -> *mut FileHandle {
    FileHandle::for_writer(std::io::sink())
}

#[no_mangle]
pub unsafe extern "C" fn new_stdout_file_handle() -> *mut FileHandle {
    FileHandle::for_writer(std::io::stdout())
}

#[no_mangle]
pub unsafe extern "C" fn new_file_handle_from_path(path: *const c_char) -> *mut FileHandle {
    let path = match CStr::from_ptr(path).to_str() {
        Ok(p) => p,
        Err(_) => return ptr::null_mut(),
    };

    let f = match File::create(path) {
        Ok(f) => f,
        Err(_) => return ptr::null_mut(),
    };

    FileHandle::for_writer(f)
}

#[no_mangle]
pub unsafe extern "C" fn file_handle_destroy(handle: *mut FileHandle) {
    let destructor = (*handle).destroy;
    destructor(handle);
}

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

#[no_mangle]
pub unsafe extern "C" fn file_handle_flush(handle: *mut FileHandle) -> c_int {
    let flush = (*handle).flush;

    match flush(handle) {
        Ok(_) => 0,
        Err(e) => e.raw_os_error().unwrap_or(-1),
    }
}
