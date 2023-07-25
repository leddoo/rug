use sti::alloc::{Alloc, GlobalAlloc};
use sti::vec::Vec;
use sti::simd::*;

use core::marker::PhantomData;
use core::ptr::NonNull;


pub struct ImgImpl<'a, T: Copy, const MUT: bool> {
    data:    *mut T,
    len:     usize,
    size:    U32x2,
    stride:  usize,
    phantom: PhantomData<(&'a (), fn(T) -> T)>,
}

pub type Img<'a, T>    = ImgImpl<'a, T, false>;
pub type ImgMut<'a, T> = ImgImpl<'a, T, true>;


impl<'a, T: Copy, const MUT: bool> ImgImpl<'a, T, MUT> {
    #[inline(always)]
    pub fn data(&self) -> &[T] {
        unsafe { core::slice::from_raw_parts(self.data, self.len) }
    }

    #[inline(always)]
    pub fn size(&self) -> U32x2 { self.size }

    #[inline(always)]
    pub fn width(&self) -> u32 { self.size.x() }

    #[inline(always)]
    pub fn height(&self) -> u32 { self.size.y() }

    #[inline(always)]
    pub fn stride(&self) -> usize { self.stride }


    #[track_caller]
    #[inline(always)]
    pub fn truncate(&mut self, new_size: [u32; 2]) {
        let [w, h] = new_size;
        assert!(w <= self.width() && h <= self.height());
        self.size = new_size.into();
    }


    #[track_caller]
    #[inline(always)]
    pub fn read_n<const N: usize>(&self, x: usize, y: usize) -> [T; N] {
        assert!(x + N <= self.width() as usize && y < self.height() as usize);
        unsafe {
            let ptr = self.data().as_ptr().add(y*self.stride + x) as *const [T; N];
            return ptr.read_unaligned();
        }
    }
}

impl<'a, T: Copy> ImgImpl<'a, T, true> {
    #[inline(always)]
    pub fn data_mut(&mut self) -> &mut [T] { 
        unsafe { core::slice::from_raw_parts_mut(self.data, self.len) }
    }

    #[track_caller]
    #[inline(always)]
    pub fn write_n<const N: usize>(&mut self, x: usize, y: usize, vs: [T; N]) {
        assert!(x + N <= self.width() as usize && y < self.height() as usize);
        unsafe {
            let ptr = self.data_mut().as_mut_ptr().add(y*self.stride + x) as *mut [T; N];
            return ptr.write_unaligned(vs);
        }
    }
}

impl<'a, T: Copy, const MUT: bool> core::ops::Index<(usize, usize)> for ImgImpl<'a, T, MUT> {
    type Output = T;

    #[inline(always)]
    fn index(&self, (x, y): (usize, usize)) -> &Self::Output {
        assert!(x < self.width() as usize && y < self.height() as usize);
        let stride = self.stride();
        &self.data()[y*stride + x]
    }
}

impl<'a, T: Copy> core::ops::IndexMut<(usize, usize)> for ImgImpl<'a, T, true> {
    #[inline(always)]
    fn index_mut(&mut self, (x, y): (usize, usize)) -> &mut Self::Output {
        assert!(x < self.width() as usize && y < self.height() as usize);
        let stride = self.stride();
        &mut self.data_mut()[y*stride + x]
    }
}



// @temp
use core::alloc::Layout;

pub struct Box<T: ?Sized, A: Alloc> {
    data:  NonNull<T>,
    alloc: A,
}

impl<T, A: Alloc> Box<T, A> {
    pub fn try_new_in(value: T, alloc: A) -> Option<Self> {
        let layout = Layout::for_value(&value);
        let data = alloc.alloc(layout)?;
        Some(Box { data: data.cast(), alloc })
    }

    #[track_caller]
    #[inline(always)]
    pub fn new_in(value: T, alloc: A) -> Self {
        Self::try_new_in(value, alloc).unwrap()
    }
}

impl<T> Box<T, GlobalAlloc> {
    #[track_caller]
    #[inline(always)]
    pub fn new(value: T) -> Self {
        Box::new_in(value, GlobalAlloc)
    }
}


impl<T, A: Alloc> Box<[T], A> {
    pub fn try_new_slice_in(value: T, len: usize, alloc: A) -> Option<Self>  where T: Copy {
        let layout = Layout::array::<T>(len).ok()?;
        let data: NonNull<T> = alloc.alloc(layout)?.cast();

        for i in 0..len {
            unsafe { data.as_ptr().add(i).write(value) };
        }

        let data = NonNull::slice_from_raw_parts(data, len);
        Some(Box { data, alloc })
    }
}

impl<T, A: Alloc> Box<[T], A> {
    #[track_caller]
    #[inline(always)]
    pub fn new_slice_in(value: T, len: usize, alloc: A) -> Self  where T: Copy {
        Box::try_new_slice_in(value, len, alloc).unwrap()
    }
}


impl<T: ?Sized, A: Alloc> Box<T, A> {
    #[inline(always)]
    pub fn as_ptr(&self) -> *const T { self.data.as_ptr() }

    #[inline(always)]
    pub fn as_mut_ptr(&mut self) -> *mut T { self.data.as_ptr() }

    #[inline(always)]
    pub fn alloc(&self) -> &A { &self.alloc }
}


impl<T: ?Sized, A: Alloc> core::ops::Deref for Box<T, A> {
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        unsafe { self.data.as_ref() }
    }
}

impl<T: ?Sized, A: Alloc> core::ops::DerefMut for Box<T, A> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.data.as_mut() }
    }
}

impl<T: ?Sized, A: Alloc> Drop for Box<T, A> {
    fn drop(&mut self) {
        unsafe {
            let layout = Layout::for_value(self.data.as_ref());
            self.alloc.free(self.data.cast(), layout);
        }
    }
}


pub struct Image<T: Copy, A: Alloc> {
    data:   Vec<T, A>,
    size:   U32x2,
    stride: usize,
}

impl<T: Copy, A: Alloc> Image<T, A> {
    fn resize_and_clear_vec(vec: &mut Vec<T, A>, new_size: [u32; 2], clear: T) -> usize {
        let [w, h] = new_size;
        let new_len = (w as usize).checked_mul(h as usize).unwrap();

        vec.reserve_exact(new_len);
        unsafe {
            let base = vec.as_mut_ptr();
            for i in 0..new_len {
                base.add(i).write(clear);
            }
            vec.set_len(new_len);
        }

        return w as usize;
    }

    #[track_caller]
    pub fn with_clear_in(size: [u32; 2], clear: T, alloc: A) -> Self {
        let mut data = Vec::new_in(alloc);
        let stride = Self::resize_and_clear_vec(&mut data, size, clear);
        Image { data, size: size.into(), stride }
    }

    #[track_caller]
    pub fn new_in(size: [u32; 2], alloc: A) -> Self  where T: Default {
        Self::with_clear_in(size, T::default(), alloc)
    }
}

impl<T: Copy> Image<T, GlobalAlloc> {
    #[track_caller]
    #[inline(always)]
    pub fn with_clear(size: [u32; 2], clear: T) -> Self {
        Image::with_clear_in(size, clear, GlobalAlloc)
    }

    #[track_caller]
    #[inline(always)]
    pub fn new(size: [u32; 2]) -> Self  where T: Default {
        Image::new_in(size, GlobalAlloc)
    }
}


impl<T: Copy, A: Alloc> Image<T, A> {
    #[inline(always)]
    pub fn data(&self) -> &[T] { self.data.as_ref() }

    #[inline(always)]
    pub fn data_mut(&mut self) -> &mut [T] { self.data.as_mut() }


    #[inline(always)]
    pub fn view(&self) -> Img<T> {
        let len = self.data.len();
        Img {
            data:    self.data.as_ptr() as *mut T, // is only used as `*const T`.
            len,
            size:    self.size,
            stride:  self.stride,
            phantom: PhantomData,
        }
    }

    #[inline(always)]
    pub fn view_mut(&mut self) -> ImgMut<T> {
        let len = self.data.len(); // stacked borrows.
        ImgMut {
            data:    self.data.as_mut_ptr(),
            len,
            size:    self.size,
            stride:  self.stride,
            phantom: PhantomData,
        }
    }


    #[inline(always)]
    pub fn size(&self) -> U32x2 { self.size }

    #[inline(always)]
    pub fn width(&self) -> u32 { self.size.x() }

    #[inline(always)]
    pub fn height(&self) -> u32 { self.size.y() }

    #[inline(always)]
    pub fn stride(&self) -> usize { self.stride }


    #[track_caller]
    #[inline(always)]
    pub fn read_n<const N: usize>(&self, x: usize, y: usize) -> [T; N] {
        assert!(x + N <= self.width() as usize && y < self.height() as usize);
        unsafe {
            let ptr = self.data().as_ptr().add(y*self.stride + x) as *const [T; N];
            return ptr.read_unaligned();
        }
    }

    #[track_caller]
    #[inline(always)]
    pub fn write_n<const N: usize>(&mut self, x: usize, y: usize, vs: [T; N]) {
        assert!(x + N <= self.width() as usize && y < self.height() as usize);
        unsafe {
            let ptr = self.data_mut().as_mut_ptr().add(y*self.stride + x) as *mut [T; N];
            return ptr.write_unaligned(vs);
        }
    }


    pub fn resize_and_clear(&mut self, new_size: [u32; 2], clear: T) {
        self.stride = Self::resize_and_clear_vec(&mut self.data, new_size, clear);
        self.size = new_size.into();
    }
}

impl<T: Copy, A: Alloc> core::ops::Index<(usize, usize)> for Image<T, A> {
    type Output = T;

    #[inline(always)]
    fn index(&self, (x, y): (usize, usize)) -> &Self::Output {
        assert!(x < self.width() as usize && y < self.height() as usize);
        let stride = self.stride();
        &self.data()[y*stride + x]
    }
}

impl<T: Copy, A: Alloc> core::ops::IndexMut<(usize, usize)> for Image<T, A> {
    #[inline(always)]
    fn index_mut(&mut self, (x, y): (usize, usize)) -> &mut Self::Output {
        assert!(x < self.width() as usize && y < self.height() as usize);
        let stride = self.stride();
        &mut self.data_mut()[y*stride + x]
    }
}

