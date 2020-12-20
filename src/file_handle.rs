use std::{
    alloc::Layout,
    any::{Any, TypeId},
    fmt::{Display, Formatter},
    io::{Error, ErrorKind, Write},
    sync::Mutex,
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
    pub(crate) poisoned: bool,
    pub(crate) destroy: unsafe fn(*mut FileHandle),
    pub(crate) write: unsafe fn(*mut FileHandle, &[u8]) -> Result<usize, Error>,
    pub(crate) flush: unsafe fn(*mut FileHandle) -> Result<(), Error>,
}

impl FileHandle {
    /// Create a new [`FileHandle`] that wraps a Rust [`std::io::Write`]r.
    pub fn for_writer<W>(writer: W) -> *mut FileHandle
    where
        W: Write + Send + Sync + 'static,
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
        let layout = Layout::new::<Repr<W>>();
        let type_id = TypeId::of::<W>();

        FileHandle {
            layout,
            type_id,
            poisoned: false,
            destroy: destroy::<W>,
            write: write::<W>,
            flush: flush::<W>,
        }
    }
}

// SAFETY: The following functions can only be used when `handle` is actually a
// `*mut Repr<W>`.

unsafe fn destroy<W>(handle: *mut FileHandle) {
    if handle.is_null() {
        return;
    }

    let repr = handle as *mut Repr<W>;

    // Safety: If there was a panic it is no longer safe to call the object's
    // destructor (it's probably FUBAR), but we can still reclaim the memory
    // used by the original allocation.

    if (*handle).poisoned {
        let layout = (*handle).layout;
        std::alloc::dealloc(repr.cast(), layout);
    } else {
        let _ = Box::from_raw(repr);
    }
}

macro_rules! auto_poison {
    ($handle:expr, $body:block) => {{
        if (*$handle).poisoned {
            Err(Error::new(
                std::io::ErrorKind::InvalidData,
                "A panic occurred and this object is now poisoned",
            ))
        } else {
            let got = std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                move || $body,
            ));
            match got {
                Ok(value) => value,
                Err(payload) => {
                    (*$handle).poisoned = true;
                    Err(Error::new(ErrorKind::Other, Poisoned::from(payload)))
                },
            }
        }
    }};
}

unsafe fn write<W: Write>(
    handle: *mut FileHandle,
    data: &[u8],
) -> Result<usize, Error> {
    auto_poison!(handle, {
        let repr = &mut *(handle as *mut Repr<W>);
        repr.writer.write(data)
    })
}

unsafe fn flush<W: Write>(handle: *mut FileHandle) -> Result<(), Error> {
    auto_poison!(handle, {
        let repr = &mut *(handle as *mut Repr<W>);
        repr.writer.flush()
    })
}

#[derive(Debug)]
struct Poisoned(Mutex<Box<dyn Any + Send + 'static>>);

impl From<Box<dyn Any + Send + 'static>> for Poisoned {
    fn from(payload: Box<dyn Any + Send + 'static>) -> Self {
        Poisoned(Mutex::new(payload))
    }
}

impl std::error::Error for Poisoned {}

impl Display for Poisoned {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let payload = self.0.lock().unwrap();

        if let Some(s) = payload.downcast_ref::<&str>() {
            write!(f, "A panic occurred: {}", s)
        } else if let Some(s) = payload.downcast_ref::<String>() {
            write!(f, "A panic occurred: {}", s)
        } else {
            write!(f, "A panic occurred")
        }
    }
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
