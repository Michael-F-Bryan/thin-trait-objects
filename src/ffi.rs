pub use crate::external::{new_file_handle_builder, FileHandleBuilder};

use crate::FileHandle;
use std::{
    ffi::CStr,
    fs::File,
    os::raw::{c_char, c_int},
    panic, ptr,
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
pub unsafe extern "C" fn new_file_handle_from_path(
    path: *const c_char,
) -> *mut FileHandle {
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

/// Free the [`FileHandle`], calling any destructors and cleaning up any
/// resources being used.
#[no_mangle]
pub unsafe extern "C" fn file_handle_destroy(handle: *mut FileHandle) {
    let _ = panic::catch_unwind(|| {
        let destructor = (*handle).destroy;
        destructor(handle);
    });
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

    match panic::catch_unwind(|| write(handle, data)) {
        Ok(Ok(bytes_written)) => bytes_written as c_int,
        Ok(Err(e)) => -e.raw_os_error().unwrap_or(1),
        Err(_) => return -1,
    }
}

/// Flush this output stream, ensuring that all intermediately buffered contents
/// reach their destination.
///
/// Returns `0` on success or a negative value on failure.
#[no_mangle]
pub unsafe extern "C" fn file_handle_flush(handle: *mut FileHandle) -> c_int {
    let flush = (*handle).flush;

    match panic::catch_unwind(|| flush(handle)) {
        Ok(Ok(_)) => 0,
        Ok(Err(e)) => -e.raw_os_error().unwrap_or(1),
        Err(_) => -1,
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::{
        io::{Error, Write},
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc, Mutex,
        },
    };

    struct NotifyOnDrop(Arc<AtomicBool>);

    impl Drop for NotifyOnDrop {
        fn drop(&mut self) { self.0.store(true, Ordering::SeqCst); }
    }

    impl Write for NotifyOnDrop {
        fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> { todo!() }

        fn flush(&mut self) -> std::io::Result<()> { todo!() }
    }

    #[test]
    fn writer_destructor_is_always_called() {
        let was_dropped = Arc::new(AtomicBool::new(false));
        let file_handle =
            FileHandle::for_writer(NotifyOnDrop(Arc::clone(&was_dropped)));
        assert!(!file_handle.is_null());

        unsafe {
            file_handle_destroy(file_handle);
        }

        assert!(was_dropped.load(Ordering::SeqCst));
    }

    #[test]
    fn create_null_file_handle_and_destroy_it() {
        unsafe {
            let handle = FileHandle::for_writer(std::io::sink());
            assert!(!handle.is_null());

            file_handle_destroy(handle);
        }
    }

    #[test]
    fn detect_failed_write() {
        struct DodgyWriter;
        impl Write for DodgyWriter {
            fn write(&mut self, _data: &[u8]) -> Result<usize, Error> {
                Err(Error::from_raw_os_error(42))
            }

            fn flush(&mut self) -> Result<(), Error> { Ok(()) }
        }

        unsafe {
            let handle = FileHandle::for_writer(DodgyWriter);
            let msg = "Hello, World!";

            let ret =
                file_handle_write(handle, msg.as_ptr() as _, msg.len() as _);
            assert_eq!(ret, -42);

            file_handle_destroy(handle);
        }
    }

    #[derive(Debug, Clone, Default)]
    pub(crate) struct SharedBuffer(pub(crate) Arc<Mutex<Vec<u8>>>);

    impl Write for SharedBuffer {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().write(buf)
        }

        fn flush(&mut self) -> std::io::Result<()> {
            self.0.lock().unwrap().flush()
        }
    }

    #[test]
    fn write_to_shared_buffer() {
        let msg = "Hello, World!";
        let buffer = SharedBuffer::default();

        unsafe {
            let handle = FileHandle::for_writer(buffer.clone());
            assert!(!handle.is_null());

            let ret = file_handle_write(
                handle,
                msg.as_ptr() as *const _,
                msg.len() as _,
            );
            assert_eq!(ret, msg.len() as _);

            let ret = file_handle_flush(handle);
            assert_eq!(ret, 0);

            file_handle_destroy(handle);
        }

        let written = buffer.0.lock().unwrap();
        let got = String::from_utf8(written.clone()).unwrap();

        assert_eq!(got, msg);
    }
}
