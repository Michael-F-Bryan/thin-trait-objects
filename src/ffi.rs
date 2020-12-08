pub use crate::external::{new_file_handle_builder, FileHandleBuilder};

use crate::FileHandle;
use std::{
    ffi::CStr,
    fs::File,
    os::raw::{c_char, c_int},
    ptr,
};

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
///
/// The return value is negative when writing fails.
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
        Err(e) => -e.raw_os_error().unwrap_or(1),
    }
}

/// Flush this output stream, ensuring that all intermediately buffered contents
/// reach their destination.
///
/// Returns `0` on success or a negative value on failure.
#[no_mangle]
pub unsafe extern "C" fn file_handle_flush(handle: *mut FileHandle) -> c_int {
    let flush = (*handle).flush;

    match flush(handle) {
        Ok(_) => 0,
        Err(e) => -e.raw_os_error().unwrap_or(1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Error, Write};

    #[test]
    fn detect_failed_write() {
        struct DodgyWriter;
        impl Write for DodgyWriter {
            fn write(&mut self, _data: &[u8]) -> Result<usize, Error> {
                Err(Error::from_raw_os_error(42))
            }

            fn flush(&mut self) -> Result<(), Error> {
                Ok(())
            }
        }

        unsafe {
            let handle = FileHandle::for_writer(DodgyWriter);
            let msg = "Hello, World!";

            let ret = file_handle_write(handle, msg.as_ptr() as _, msg.len() as _);
            assert_eq!(ret, -42);

            file_handle_destroy(handle);
        }
    }
}
