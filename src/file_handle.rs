use std::io::{Error, Write};
use std::{alloc::Layout, any::TypeId};

/// A FFI-safe version of the trait object, [`dyn std::io::Write`][Write].
///
/// A [`FileHandle`] is an abstract base class containing just the object's
/// vtable. It can only be created safely via the [`FileHandle::for_writer()`]
/// constructor.
///
/// # Safety
///
/// A [`FileHandle`] is an unsized type and must always be kept behind a
/// pointer. Copying a [`FileHandle`] to the stack will result in a phenomenon
/// called [*Object Slicing*][slicing], corrupting the `FileHandle`.
///
/// [slicing]: https://stackoverflow.com/questions/274626/what-is-object-slicing
#[derive(Clone)]
#[repr(C)]
pub struct FileHandle {
    pub(crate) layout: Layout,
    pub(crate) type_id: TypeId,
    pub(crate) destroy: unsafe fn(*mut FileHandle),
    pub(crate) write: unsafe fn(*mut FileHandle, &[u8]) -> Result<usize, Error>,
    pub(crate) flush: unsafe fn(*mut FileHandle) -> Result<(), Error>,
}

impl FileHandle {
    /// Create a new [`FileHandle`] that wraps a Rust [`std::io::Write`]r.
    pub fn for_writer<W>(writer: W) -> *mut FileHandle
    where
        W: Write + 'static,
    {
        let repr = Repr {
            base: FileHandle::vtable::<W>(),
            writer,
        };

        let boxed = Box::into_raw(Box::new(repr));

        unsafe {
            // Safety: A pointer to the first field on a #[repr(C)] struct has
            // the same address as the struct itself
            &mut (*boxed).base
        }
    }

    fn vtable<W: Write + 'static>() -> FileHandle {
        let layout = Layout::new::<W>();
        let type_id = TypeId::of::<W>();

        FileHandle {
            layout,
            type_id,
            destroy: destroy::<W>,
            write: write::<W>,
            flush: flush::<W>,
        }
    }
}

unsafe fn destroy<W>(handle: *mut FileHandle) {
    let repr = handle as *mut Repr<W>;
    let _ = Box::from_raw(repr);
}

unsafe fn write<W: Write>(handle: *mut FileHandle, data: &[u8]) -> Result<usize, Error> {
    let repr = &mut *(handle as *mut Repr<W>);
    repr.writer.write(data)
}

unsafe fn flush<W: Write>(handle: *mut FileHandle) -> Result<(), Error> {
    let repr = &mut *(handle as *mut Repr<W>);
    repr.writer.flush()
}

/// The "child class" which inherits from [`FileHandle`] and.
#[repr(C)]
pub(crate) struct Repr<W> {
    // Safety: The FileHandle must be the first field so we can cast between
    // *mut Repr<W> and *mut FileHandle
    pub(crate) base: FileHandle,
    pub(crate) writer: W,
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::ffi;
    use std::sync::{Arc, Mutex};

    use super::*;

    #[test]
    fn create_null_file_handle_and_destroy_it() {
        unsafe {
            let handle = FileHandle::for_writer(std::io::sink());
            assert!(!handle.is_null());

            ffi::file_handle_destroy(handle);
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

            let ret = ffi::file_handle_write(handle, msg.as_ptr() as *const _, msg.len() as _);
            assert_eq!(ret, msg.len() as _);

            let ret = ffi::file_handle_flush(handle);
            assert_eq!(ret, 0);

            ffi::file_handle_destroy(handle);
        }

        let written = buffer.0.lock().unwrap();
        let got = String::from_utf8(written.clone()).unwrap();

        assert_eq!(got, msg);
    }
}
