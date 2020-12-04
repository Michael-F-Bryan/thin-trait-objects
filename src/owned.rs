use std::{any::TypeId, io::Write, ptr::NonNull};

use crate::{file_handle::Repr, FileHandle};

#[derive(Debug)]
pub struct OwnedFileHandle(NonNull<FileHandle>);

impl OwnedFileHandle {
    pub fn new<W: Write + 'static>(writer: W) -> Self {
        unsafe {
            let handle = FileHandle::for_writer(writer);
            assert!(!handle.is_null());
            OwnedFileHandle::from_raw(handle)
        }
    }

    pub unsafe fn from_raw(handle: *mut FileHandle) -> Self {
        OwnedFileHandle(NonNull::new_unchecked(handle))
    }

    pub fn is<W: 'static>(&self) -> bool {
        unsafe { self.0.as_ref().type_id == TypeId::of::<W>() }
    }

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

    pub fn downcast<W: 'static>(self) -> Result<W, Self> {
        if self.is::<W>() {
            unsafe {
                // Safety: We just did a type check
                let repr = self.0.as_ptr() as *mut Repr<W>;
                // make sure we don't run the OwnedFileHandle's destructor
                std::mem::forget(self);

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
            crate::ffi::file_handle_destroy(self.0.as_ptr());
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::file_handle::tests::SharedBuffer;

    use super::*;

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
