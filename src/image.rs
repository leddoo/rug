extern crate alloc;
use alloc::{
    alloc::{Allocator, Global},
    boxed::Box,
    vec::Vec,
};

use crate::wide::*;


pub type Mask<'a>   = Image_a_f32<'a>;
pub type Target<'a> = Image_rgba_f32x8<'a>;



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

macro_rules! image_impl_index {
    ($name: ident, $T: ty) => {
        impl<'a> core::ops::Index<(usize, usize)> for $name<'a> {
            type Output = $T;

            #[inline(always)]
            fn index(&self, index: (usize, usize)) -> & $T {
                let (x, y) = index;
                &self.data[y*self.stride() + x]
            }
        }

        impl<'a> core::ops::IndexMut<(usize, usize)> for $name<'a> {
            #[inline(always)]
            fn index_mut(&mut self, index: (usize, usize)) -> &mut $T {
                let (x, y) = index;
                &mut self.data[y*self.stride() + x]
            }
        }

        impl<'a> core::ops::Index<usize> for $name<'a> {
            type Output = $T;

            #[inline(always)]
            fn index(&self, index: usize) -> & $T {
                &self.data[index]
            }
        }

        impl<'a> core::ops::IndexMut<usize> for $name<'a> {
            #[inline(always)]
            fn index_mut(&mut self, index: usize) -> &mut $T {
                &mut self.data[index]
            }
        }
    };
}



#[allow(non_camel_case_types)]
pub struct Image_a_f32<'a> {
    data:   Box<[f32], &'a dyn Allocator>,
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
    pub fn read8(&self, x: usize, y: usize) -> F32x8 {
        assert!(x + 8 <= self.width() as usize && y < self.height() as usize);
        unsafe {
            let ptr = self.data.as_ptr().add(y*self.stride + x);
            let ptr = ptr as *const F32x8;
            ptr.read_unaligned()
        }
    }


    pub fn clear(&mut self, value: f32) {
        for v in self.data.iter_mut() {
            *v = value;
        }
    }
}

image_impl_index!(Image_a_f32, f32);



#[allow(non_camel_case_types)]
pub struct Image_rgba_f32x8<'a> {
    data:   Box<[(F32x8, F32x8, F32x8, F32x8)], &'a dyn Allocator>,
    bounds: U32x2,
    stride: usize,
}

impl<'a> Image_rgba_f32x8<'a> {
    pub fn new(width: u32, height: u32) -> Image_rgba_f32x8<'a> {
        Image_rgba_f32x8::new_in(width, height, &Global)
    }

    pub fn new_in(width: u32, height: u32, allocator: &'a dyn Allocator) -> Image_rgba_f32x8<'a> {
        let stride = ((width + 7) / 8 ) as usize;
        Image_rgba_f32x8 {
            data:   new_slice_box(stride * height as usize, allocator),
            bounds: [width, height].into(),
            stride,
        }
    }

    image_impl_bounds!();

    pub fn clear(&mut self, color: F32x4) {
        let r = F32x8::splat(color[0]);
        let g = F32x8::splat(color[1]);
        let b = F32x8::splat(color[2]);
        let a = F32x8::splat(color[3]);

        let value = (r, g, b, a);
        for v in self.data.iter_mut() {
            *v = value;
        }
    }
}

image_impl_index!(Image_rgba_f32x8, (F32x8, F32x8, F32x8, F32x8));



#[inline(always)]
pub fn argb_u8x8_unpack(v: U32x8) -> (F32x8, F32x8, F32x8, F32x8) {
    let mask = U32x8::splat(0xff);
    let b = v & mask;
    let g = (v >> U32x8::splat(8))  & mask;
    let r = (v >> U32x8::splat(16)) & mask;
    let a = (v >> U32x8::splat(24)) & mask;

    let scale = F32x8::splat(255.0);
    (r.cast() / scale,
     g.cast() / scale,
     b.cast() / scale,
     a.cast() / scale)
}

#[inline(always)]
pub unsafe fn argb_u8x8_pack_clamped_255(v: (F32x8, F32x8, F32x8, F32x8)) -> U32x8 {
    #[inline(always)]
    unsafe fn to_int(v: F32x8) -> U32x8 {
        core::mem::transmute(v.to_int_unchecked::<i32>())
    }

    let (r, g, b, a) = v;

    let b = to_int(b);
    let g = to_int(g) << U32x8::splat(8);
    let r = to_int(r) << U32x8::splat(16);
    let a = to_int(a) << U32x8::splat(24);
    a | r | g | b
}

#[inline(always)]
pub fn argb_u8x8_pack(v: (F32x8, F32x8, F32x8, F32x8)) -> U32x8 {
    let offset = F32x8::splat(0.5);
    let scale = F32x8::splat(255.0);
    let min = F32x8::splat(0.0);
    let max = F32x8::splat(255.0);
    let (r, g, b, a) = v;
    unsafe { argb_u8x8_pack_clamped_255((
        (scale*r + offset).clamp(min, max),
        (scale*g + offset).clamp(min, max),
        (scale*b + offset).clamp(min, max),
        (scale*a + offset).clamp(min, max),
    )) }
}

