extern crate alloc;
use alloc::alloc::{Allocator, Global};
use alloc::boxed::Box;
use alloc::{vec, vec::Vec};


pub struct RawImage<T, const CHANNELS: usize, A: Allocator = Global> {
    data:   Box<[T], A>,
    width:  u32,
    height: u32,
    stride: u32,
}

// scalar.

pub type Image   <const CHANNELS: usize, A = Global> = RawImage<u8, CHANNELS, A>;
pub type ImageF32<const CHANNELS: usize, A = Global> = RawImage<f32, CHANNELS, A>;


impl<T: Copy + Default, const CHANNELS: usize> RawImage<T, CHANNELS> {
    pub fn new(width: u32, height: u32) -> RawImage<T, CHANNELS> {
        let stride = width;
        let data = vec![T::default(); (stride*height) as usize].into_boxed_slice();
        RawImage::<T, CHANNELS, Global> {
            data, width, height, stride,
        }
    }
}

impl<T: Copy + Default, const CHANNELS: usize, A: Allocator> RawImage<T, CHANNELS, A> {
    pub fn new_in(width: u32, height: u32, allocator: A) -> RawImage<T, CHANNELS, A> {
        let stride = width;
        let data = {
            let mut data = Vec::new_in(allocator);
            data.resize((stride*height) as usize, T::default());
            data.into_boxed_slice()
        };
        RawImage::<T, CHANNELS, A> {
            data, width, height, stride,
        }
    }

    pub fn width(&self)  -> u32 { self.width  }
    pub fn height(&self) -> u32 { self.height }
    pub fn stride(&self) -> u32 { self.stride }

    pub fn channel_offset(&self) -> u32 { self.height * self.stride }

    pub fn raw_index(&self, index: u32) -> T {
        debug_assert!(index % self.stride < self.width);
        debug_assert!(index / self.stride < self.height);
        self.data[index as usize]
    }

    pub fn raw_index_mut(&mut self, index: u32) -> &mut T {
        debug_assert!(index % self.stride < self.width);
        debug_assert!(index / self.stride < self.height);
        &mut self.data[index as usize]
    }

    pub unsafe fn raw_index_unck(&self, index: u32) -> T {
        *self.data.get_unchecked(index as usize)
    }

    pub unsafe fn raw_index_mut_unck(&mut self, index: u32) -> &mut T {
        self.data.get_unchecked_mut(index as usize)
    }


    pub fn truncate_width(&mut self, new_width: u32) {
        assert!(new_width <= self.width);
        self.width = new_width;
    }

    pub fn truncate_height(&mut self, new_height: u32) {
        assert!(new_height <= self.height);
        self.height = new_height;
    }
}



/*
pub struct RawImg<T, const CHANNELS: usize> {
    data:   T,
    width:  u32,
    height: u32,
    stride: u32,
    channel_offset: u32,
}
*/




/*
pub type Img   <'i, const CHANNELS: usize> = RawImg<&'i [u8], CHANNELS>;
pub type ImgMut<'i, const CHANNELS: usize> = RawImg<&'i mut [u8], CHANNELS>;
*/

