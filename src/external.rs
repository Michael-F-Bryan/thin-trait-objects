//! Code for creating a [`FileHandle`] which is populated by the caller.

#![allow(missing_docs)]

use crate::FileHandle;
use std::{
    alloc::Layout,
    any::TypeId,
    convert::TryInto,
    io::Error,
    os::raw::{c_char, c_int, c_void},
};

#[repr(C)]
pub struct FileHandleBuilder {
    pub file_handle: *mut FileHandle,
    pub place: *mut c_void,
}

#[no_mangle]
pub unsafe extern "C" fn new_file_handle_builder(
    size: c_int,
    alignment: c_int,
    destroy: unsafe extern "C" fn(*mut c_void),
    write: unsafe extern "C" fn(*mut c_void, *const c_char, c_int) -> c_int,
    flush: unsafe extern "C" fn(*mut c_void) -> c_int,
) -> FileHandleBuilder {
    let header_layout = Layout::new::<ExternalFileHandle>();

    // FIXME: panic in `extern "C"` code... no bueno.
    let size = size.try_into().unwrap();
    let alignment = alignment.try_into().unwrap();
    let object_layout = Layout::from_size_align(size, alignment).unwrap();

    let (overall_layout, object_offset) = header_layout.extend(object_layout).unwrap();

    // So this is a bit tricky. We're effectively trying to emulate
    // placement-new, but in Rust.
    //
    // First we'll allocate some memory for the entire object
    let ptr = std::alloc::alloc_zeroed(overall_layout);

    // now let's initialize the header part
    let ptr = ptr as *mut ExternalFileHandle;

    ptr.write(ExternalFileHandle {
        base: FileHandle {
            layout: overall_layout,
            type_id: TypeId::of::<ExternalFileHandle>(),
            destroy: destroy_external_file_handle,
            write: write_external_file_handle,
            flush: flush_external_file_handle,
        },
        object_offset,
        destroy,
        flush,
        write,
    });

    // we use the offset from earlier to find where the caller needs to
    // initialize their object
    FileHandleBuilder {
        file_handle: ptr.cast(),
        place: ptr.cast::<u8>().offset(object_offset as isize).cast(),
    }
}

#[repr(C)]
struct ExternalFileHandle {
    base: FileHandle,
    object_offset: usize,
    destroy: unsafe extern "C" fn(*mut c_void),
    write: unsafe extern "C" fn(*mut c_void, *const c_char, c_int) -> c_int,
    flush: unsafe extern "C" fn(*mut c_void) -> c_int,
}

unsafe fn object_ptr(external: *mut ExternalFileHandle) -> *mut c_void {
    (external as *mut u8).offset((*external).object_offset as isize) as *mut c_void
}

unsafe fn destroy_external_file_handle(handle: *mut FileHandle) {
    let external = handle as *mut ExternalFileHandle;

    // first we destroy the object in place
    let destroy = (*external).destroy;
    destroy(object_ptr(external));

    // then we can destroy the ExternalFileHandle
    std::ptr::drop_in_place(external);

    // and finally deallocate
    std::alloc::dealloc(external.cast(), (*external).base.layout);
}

unsafe fn write_external_file_handle(handle: *mut FileHandle, data: &[u8]) -> Result<usize, Error> {
    let external = handle as *mut ExternalFileHandle;
    let write = (*external).write;

    let ret = write(
        object_ptr(external),
        data.as_ptr() as *const _,
        data.len() as _,
    );

    if ret >= 0 {
        Ok(ret as usize)
    } else {
        Err(Error::from_raw_os_error(-ret))
    }
}
unsafe fn flush_external_file_handle(handle: *mut FileHandle) -> Result<(), Error> {
    let external = handle as *mut ExternalFileHandle;
    let flush = (*external).flush;

    let ret = flush(object_ptr(external));

    if ret >= 0 {
        Ok(())
    } else {
        Err(Error::from_raw_os_error(-ret))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ffi::{tests::SharedBuffer, *};
    use std::io::Write;

    unsafe extern "C" fn destroy_data(data: *mut c_void) {
        std::ptr::drop_in_place(data.cast::<SharedBuffer>());
    }

    unsafe extern "C" fn write_data(data: *mut c_void, buffer: *const c_char, len: c_int) -> c_int {
        let buffer = std::slice::from_raw_parts(buffer as *const u8, len as usize);

        match data
            .cast::<SharedBuffer>()
            .as_mut()
            .unwrap()
            .0
            .lock()
            .unwrap()
            .write(buffer)
        {
            Ok(bytes_written) => bytes_written as c_int,
            Err(e) => e.raw_os_error().map(|code| -code).unwrap_or(-1),
        }
    }

    unsafe extern "C" fn flush_data(data: *mut c_void) -> c_int {
        match data
            .cast::<SharedBuffer>()
            .as_mut()
            .unwrap()
            .0
            .lock()
            .unwrap()
            .flush()
        {
            Ok(_) => 0,
            Err(e) => e.raw_os_error().map(|code| -code).unwrap_or(-1),
        }
    }

    #[test]
    fn create_an_external_file_handle_and_initialize_it() {
        unsafe {
            let layout = Layout::new::<SharedBuffer>();

            let FileHandleBuilder {
                file_handle: handle,
                place,
            } = new_file_handle_builder(
                layout.size() as _,
                layout.align() as _,
                destroy_data,
                write_data,
                flush_data,
            );

            // now we need to initialize the data
            let buffer = SharedBuffer::default();
            place.cast::<SharedBuffer>().write(buffer.clone());

            // our FileHandle is now initialized so we can write to it like
            // normal
            let msg = "Hello, World!";
            let ret = file_handle_write(handle, msg.as_ptr() as *const _, msg.len() as _);
            assert_eq!(ret, 13);

            let ret = file_handle_flush(handle);
            assert_eq!(ret, 0);

            file_handle_destroy(handle);
        }
    }
}
