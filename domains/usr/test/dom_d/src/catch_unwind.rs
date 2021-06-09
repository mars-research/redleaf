//! Simple std-like catch_unwind() implementation

use core::mem::ManuallyDrop;
use core::result::Result;

#[repr(C)]
union UnwindData<F: FnOnce() -> R, R> {
    /// The function to call.
    f: ManuallyDrop<F>,

    /// The return value.
    r: ManuallyDrop<R>,

    /// The panic payload.
    p: *mut u8,
}

impl<F: FnOnce() -> R, R> UnwindData<F, R> {
    fn new(f: F) -> Self {
        Self { 
            f: ManuallyDrop::new(f),
        }
    }
}

// The try_fn that core::intrinsics::r#try expects.
// Don't call this - Pass to try()
#[inline]
fn try_fn<F: FnOnce() -> R, R>(data: *mut u8) {
    unsafe {
        let data = &mut *(data as *mut UnwindData<F, R>);
        let f = ManuallyDrop::take(&mut data.f);
        data.r = ManuallyDrop::new(f());
    }
}

// The catch_fn that core::intrinsics::r#try expects.
// Don't call this - Pass to try()
#[inline]
fn catch_fn<F: FnOnce() -> R, R>(data: *mut u8, payload: *mut u8) {
    unsafe {
        let data = &mut *(data as *mut UnwindData<F, R>);
        data.p = payload;
    }
}

pub fn catch_unwind<F: FnOnce() -> R, R>(f: F) -> Result<R, ()> {
    let data = UnwindData::new(f);
    let data_ptr = &data as *const _ as *const u8 as *mut u8;

    unsafe {
        // https://github.com/rust-lang/rust/blob/05c300144cebbf3ddaff213b7485f669a9e0b660/library/std/src/panicking.rs#L342-L348
        if core::intrinsics::r#try(try_fn::<F, R>, data_ptr, catch_fn::<F, R>) == 0 {
            Ok(ManuallyDrop::into_inner(data.r))
        } else {
            Err(())
        }
    }
}
