extern crate alloc;
use alloc::{
    alloc::{Allocator, Global},
    boxed::Box,
    vec::Vec,
};

use crate::simd::*;
use core::simd::{LaneCount, SupportedLaneCount};


pub type Mask<'a>   = Image_a_f32<'a>;
pub type Target<'a> = Image_rgba_f32x<'a, 8>;



fn new_slice_box<T: Default + Copy>(length: usize, allocator: &dyn Allocator) -> Box<[T], &dyn Allocator> {
    let mut data = Vec::new_in(allocator);
    data.resize(length, Default::default());
    data.into_boxed_slice()
}


macro_rules! image_impl_bounds {
    () => {
        #[inline(always)]
        pub fn bounds(&self) -> U32x2 { self.bounds }

        #[inline(always)]
        pub fn width(&self) -> u32 { self.bounds[0] }

        #[inline(always)]
        pub fn height(&self) -> u32 { self.bounds[1] }

        #[inline(always)]
        pub fn stride(&self) -> usize { self.stride }


        pub fn truncate_width(&mut self, new_width: u32) {
            assert!(new_width <= self.width());
            self.bounds[0] = new_width;
        }

        pub fn truncate_height(&mut self, new_height: u32) {
            assert!(new_height <= self.height());
            self.bounds[1] = new_height;
        }

        pub fn truncate(&mut self, new_width: u32, new_height: u32) {
            self.truncate_width(new_width);
            self.truncate_height(new_height);
        }
    };
}


#[allow(non_camel_case_types)]
pub struct Image_a_f32<'a> {
    pub(crate) data: Box<[f32], &'a dyn Allocator>,
    bounds: U32x2,
    stride: usize,
}

impl<'a> Image_a_f32<'a> {
    pub fn new(width: u32, height: u32) -> Image_a_f32<'a> {
        Image_a_f32::new_in(width, height, &Global)
    }

    pub fn new_in(width: u32, height: u32, allocator: &'a dyn Allocator) -> Image_a_f32<'a> {
        Image_a_f32 {
            data:   new_slice_box((width*height) as usize, allocator),
            bounds: [width, height].into(),
            stride: width as usize,
        }
    }

    image_impl_bounds!();


    #[inline(always)]
    pub fn read<const N: usize>(&self, x: usize, y: usize) -> F32x<N> where LaneCount<N>: SupportedLaneCount {
        assert!(x + N <= self.width() as usize && y < self.height() as usize);
        unsafe {
            let ptr = self.data.as_ptr().add(y*self.stride + x);
            let ptr = ptr as *const F32x<N>;
            ptr.read_unaligned()
        }
    }

    #[inline(always)]
    pub fn write<const N: usize>(&self, x: usize, y: usize, value: F32x<N>) where LaneCount<N>: SupportedLaneCount {
        assert!(x + N <= self.width() as usize && y < self.height() as usize);
        unsafe {
            let ptr = self.data.as_ptr().add(y*self.stride + x);
            let ptr = ptr as *mut F32x<N>;
            ptr.write_unaligned(value)
        }
    }


    pub fn clear(&mut self, value: f32) {
        for v in self.data.iter_mut() {
            *v = value;
        }
    }
}

impl<'a> core::ops::Index<(usize, usize)> for Image_a_f32<'a> {
    type Output = f32;

    #[inline(always)]
    fn index(&self, index: (usize, usize)) -> &Self::Output {
        let (x, y) = index;
        &self.data[y*self.stride() + x]
    }
}

impl<'a> core::ops::IndexMut<(usize, usize)> for Image_a_f32<'a> {
    #[inline(always)]
    fn index_mut(&mut self, index: (usize, usize)) -> &mut Self::Output {
        let (x, y) = index;
        &mut self.data[y*self.stride() + x]
    }
}




#[allow(non_camel_case_types)]
pub struct Image_rgba_f32x<'a, const N: usize> where LaneCount<N>: SupportedLaneCount {
    pub(crate) data: Box<[[F32x<N>; 4]], &'a dyn Allocator>,
    bounds: U32x2,
    stride: usize,
}

impl<'a, const N: usize> Image_rgba_f32x<'a, N> where LaneCount<N>: SupportedLaneCount {
    pub const fn simd_width() -> usize { N }

    pub fn new(width: u32, height: u32) -> Image_rgba_f32x<'a, N> {
        Image_rgba_f32x::new_in(width, height, &Global)
    }

    pub fn new_in(width: u32, height: u32, allocator: &'a dyn Allocator) -> Image_rgba_f32x<'a, N> {
        let stride = (width as usize + N-1) / N;
        Image_rgba_f32x {
            data:   new_slice_box(stride * height as usize, allocator),
            bounds: [width, height].into(),
            stride,
        }
    }

    image_impl_bounds!();

    pub fn clear(&mut self, color: F32x4) {
        let r = <F32x<N>>::splat(color[0]);
        let g = <F32x<N>>::splat(color[1]);
        let b = <F32x<N>>::splat(color[2]);
        let a = <F32x<N>>::splat(color[3]);

        let value = [r, g, b, a];
        for v in self.data.iter_mut() {
            *v = value;
        }
    }
}

impl<'a, const N: usize> core::ops::Index<(usize, usize)> for Image_rgba_f32x<'a, N> where LaneCount<N>: SupportedLaneCount {
    type Output = [F32x<N>; 4];

    #[inline(always)]
    fn index(&self, index: (usize, usize)) -> &Self::Output {
        let (x, y) = index;
        &self.data[y*self.stride() + x]
    }
}

impl<'a, const N: usize> core::ops::IndexMut<(usize, usize)> for Image_rgba_f32x<'a, N> where LaneCount<N>: SupportedLaneCount {
    #[inline(always)]
    fn index_mut(&mut self, index: (usize, usize)) -> &mut Self::Output {
        let (x, y) = index;
        &mut self.data[y*self.stride() + x]
    }
}



#[inline(always)]
pub fn argb_u8x_unpack<const N: usize>(v: U32x<N>) -> [F32x<N>; 4] where LaneCount<N>: SupportedLaneCount {
    let mask = <U32x<N>>::splat(0xff);
    let b = v & mask;
    let g = (v >> <U32x<N>>::splat(8))  & mask;
    let r = (v >> <U32x<N>>::splat(16)) & mask;
    let a = (v >> <U32x<N>>::splat(24)) & mask;

    let scale = <F32x<N>>::splat(255.0);
    [r.as_i32().to_f32() / scale,
     g.as_i32().to_f32() / scale,
     b.as_i32().to_f32() / scale,
     a.as_i32().to_f32() / scale]
}

#[inline(always)]
pub unsafe fn argb_u8x_pack_clamped_255<const N: usize>(v: [F32x<N>; 4]) -> U32x<N> where LaneCount<N>: SupportedLaneCount {
    let [r, g, b, a] = v;

    let b = b.to_i32_unck();
    let g = g.to_i32_unck() << <I32x<N>>::splat(8);
    let r = r.to_i32_unck() << <I32x<N>>::splat(16);
    let a = a.to_i32_unck() << <I32x<N>>::splat(24);
    (a | r | g | b).as_u32()
}

#[inline(always)]
pub fn argb_u8x_pack<const N: usize>(v: [F32x<N>; 4]) -> U32x<N> where LaneCount<N>: SupportedLaneCount {
    let offset = <F32x<N>>::splat(0.5);
    let scale = <F32x<N>>::splat(255.0);
    let min = <F32x<N>>::splat(0.0);
    let max = <F32x<N>>::splat(255.0);
    let [r, g, b, a] = v;
    unsafe { argb_u8x_pack_clamped_255([
        (scale*r + offset).clamp(min, max),
        (scale*g + offset).clamp(min, max),
        (scale*b + offset).clamp(min, max),
        (scale*a + offset).clamp(min, max),
    ]) }
}

