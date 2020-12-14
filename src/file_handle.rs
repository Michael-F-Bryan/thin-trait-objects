use std::{
    alloc::Layout,
    any::TypeId,
    io::{Error, Write},
};

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

        // Safety: A pointer to the first field on a #[repr(C)] struct has the
        // same address as the struct itself
        boxed as *mut _
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

// SAFETY: The following functions can only be used when `handle` is actually a
// `*mut Repr<W>`.

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

/// The "child class" which inherits from [`FileHandle`] and holds some type,
/// `W`, which we can write to.
#[repr(C)]
pub(crate) struct Repr<W> {
    // Safety: The FileHandle must be the first field so we can cast between
    // *mut Repr<W> and *mut FileHandle
    pub(crate) base: FileHandle,
    pub(crate) writer: W,
}
