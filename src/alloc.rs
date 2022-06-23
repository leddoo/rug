extern crate alloc;

pub use alloc::alloc::Layout as AllocLayout;

pub use alloc::alloc::Allocator as Alloc;

pub trait CopyAlloc: alloc::alloc::Allocator + Copy {}


pub use alloc::alloc::Global as GlobalAlloc;

impl CopyAlloc for GlobalAlloc {}


pub use alloc::boxed::Box;
pub use alloc::vec::Vec;


#[inline(always)]
pub fn align_pointer<T>(p: *const T) -> *const T {
    let align = core::mem::align_of::<T>();
    align_pointer_to(p, align)
}

#[inline(always)]
pub fn align_pointer_to<T>(p: *const T, align: usize) -> *const T {
    ((p as usize + align-1) / align * align) as *const T
}

