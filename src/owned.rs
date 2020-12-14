use crate::{file_handle::Repr, FileHandle};
use std::{any::TypeId, io::Write, ptr::NonNull};

/// An owned wrapper around a [`*mut FileHandle`][FileHandle] for use in Rust
/// code.
///
/// A [`FileHandle`] can be thought of as a `dyn Write`, making
/// [`OwnedFileHandle`] the FFI-safe equivalent of `Box<dyn Write>`. The primary
/// difference is that an `OwnedFileHandle` is the size of a single pointer
/// while `Box<dyn Write>` is a fat pointer that is twice the size.
///
/// ```rust
/// # use std::{mem::size_of, io::Write};
/// # use thin_trait_objects::OwnedFileHandle;
/// assert_eq!(size_of::<OwnedFileHandle>(), size_of::<*mut u8>());
/// assert_ne!(size_of::<Box<dyn Write>>(), size_of::<*mut u8>());
///
/// The "Null Pointer Optimisation" also holds
/// assert_eq!(size_of::<Option<OwnedFileHandle>>(), size_of::<OwnedFileHandle>());
/// ```
#[derive(Debug)]
#[repr(transparent)]
pub struct OwnedFileHandle(NonNull<FileHandle>);

impl OwnedFileHandle {
    /// Create a new [`OwnedFileHandle`] which wraps some [`Write`]r.
    pub fn new<W: Write + 'static>(writer: W) -> Self {
        unsafe {
            let handle = FileHandle::for_writer(writer);
            assert!(!handle.is_null());
            OwnedFileHandle::from_raw(handle)
        }
    }

    /// Create an [`OwnedFileHandle`] from a `*mut FileHandle`, taking
    /// ownership of the [`FileHandle`].
    ///
    /// # Safety
    ///
    /// Ownership of the `handle` is given to the [`OwnedFileHandle`] and the
    /// original pointer may no longer be used.
    ///
    /// The `handle` must be a non-null pointer which points to a valid
    /// `FileHandle`.
    pub unsafe fn from_raw(handle: *mut FileHandle) -> Self {
        debug_assert!(!handle.is_null());
        OwnedFileHandle(NonNull::new_unchecked(handle))
    }

    /// Consume the [`OwnedFileHandle`] and get a `*mut FileHandle` that can be
    /// used from native code.
    pub fn into_raw(self) -> *mut FileHandle {
        let ptr = self.0.as_ptr();
        std::mem::forget(self);
        ptr
    }

    /// Check if the object pointed to by a [`OwnedFileHandle`] has type `W`.
    pub fn is<W: 'static>(&self) -> bool {
        unsafe { self.0.as_ref().type_id == TypeId::of::<W>() }
    }

    /// Returns a reference to the boxed value if it is of type `T`, or
    /// `None` if it isn't.
    pub fn downcast_ref<W: 'static>(&self) -> Option<&W> {
        if self.is::<W>() {
            unsafe {
                // Safety: We just did a type check
                let repr = self.0.as_ptr() as *const Repr<W>;
                Some(&(*repr).writer)
            }
        } else {
            None
        }
    }

    /// Returns a mutable reference to the boxed value if it is of type `T`, or
    /// `None` if it isn't.
    pub fn downcast_mut<W: 'static>(&mut self) -> Option<&mut W> {
        if self.is::<W>() {
            unsafe {
                // Safety: We just did a type check
                let repr = self.0.as_ptr() as *mut Repr<W>;
                Some(&mut (*repr).writer)
            }
        } else {
            None
        }
    }

    /// Attempt to downcast the [`OwnedFileHandle`] to a concrete type and
    /// extract it.
    pub fn downcast<W: 'static>(self) -> Result<W, Self> {
        if self.is::<W>() {
            unsafe {
                let ptr = self.into_raw();
                // Safety: We just did a type check
                let repr: *mut Repr<W> = ptr.cast();

                let unboxed = Box::from_raw(repr);
                Ok(unboxed.writer)
            }
        } else {
            Err(self)
        }
    }
}

impl Write for OwnedFileHandle {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        unsafe {
            let ptr = self.0.as_mut();
            (ptr.write)(ptr, buf)
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        unsafe {
            let ptr = self.0.as_mut();
            (ptr.flush)(ptr)
        }
    }
}

impl Drop for OwnedFileHandle {
    fn drop(&mut self) {
        unsafe {
            let ptr = self.0.as_mut();
            (ptr.destroy)(ptr);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ffi::tests::SharedBuffer;
    use std::sync::Arc;

    #[test]
    fn downcast_ref() {
        let buffer = SharedBuffer::default();
        let handle = OwnedFileHandle::new(buffer.clone());

        let got = handle.downcast_ref::<SharedBuffer>().unwrap();
        assert!(Arc::ptr_eq(&got.0, &buffer.0));
    }

    #[test]
    fn downcast_mut() {
        let buffer = SharedBuffer::default();
        let mut handle = OwnedFileHandle::new(buffer.clone());

        let got = handle.downcast_mut::<SharedBuffer>().unwrap();
        assert!(Arc::ptr_eq(&got.0, &buffer.0));
    }

    #[test]
    fn downcast_owned_doesnt_destroy_twice() {
        let handle = OwnedFileHandle::new(std::io::sink());

        let got = handle.downcast::<SharedBuffer>();
        assert!(got.is_err());
        let handle = got.unwrap_err();

        let got = handle.downcast::<std::io::Sink>();
        assert!(got.is_ok());
    }
}
