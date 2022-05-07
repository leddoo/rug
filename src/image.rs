extern crate alloc;
use alloc::{
    alloc::{Allocator, Global},
    boxed::Box,
    vec::Vec,
};

use crate::wide::*;


#[allow(non_camel_case_types)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    a_u8,
    a_f32,
    rgba_u8_soa,
    argb_u32,
}

impl ImageFormat {
    // returns (size, stride).
    // TODO: use math utils.
    pub fn image_size(self, width: u32, height: u32) -> (u32, u32) {
        match self {
            ImageFormat::a_u8 => {
                let stride = (width + 31) / 32;
                (height * stride, stride)
            },

            ImageFormat::a_f32 => {
                let stride = (4*width + 31) / 32;
                (height * stride, stride)
            },

            ImageFormat::rgba_u8_soa => {
                let stride = (width + 31) / 32;
                (4 * height * stride, stride)
            },

            ImageFormat::argb_u32 => {
                let stride = (4*width + 31) / 32;
                (height * stride, stride)
            },
        }
    }
}


#[inline(always)]
pub fn argb_unpack(v: U32x8) -> (F32x8, F32x8, F32x8, F32x8) {
    let mask = U32x8::splat(0xff);
    let b = v & mask;
    let g = (v >> U32x8::splat(8))  & mask;
    let r = (v >> U32x8::splat(16)) & mask;
    let a = (v >> U32x8::splat(24)) & mask;

    let scale = F32x8::splat(255.0);
    (r.cast() / scale, g.cast() / scale, b.cast() / scale, a.cast() / scale)
}

#[inline(always)]
pub unsafe fn argb_pack_clamped(r: F32x8, g: F32x8, b: F32x8, a: F32x8) -> U32x8 {
    unsafe fn to_int(v: F32x8) -> U32x8 {
        core::mem::transmute(v.to_int_unchecked::<i32>())
    }

    let scale = F32x8::splat(255.0);
    let b = to_int(scale * b);
    let g = to_int(scale * g) << U32x8::splat(8);
    let r = to_int(scale * r) << U32x8::splat(16);
    let a = to_int(scale * a) << U32x8::splat(24);
    a | r | g | b
}

#[inline(always)]
pub fn argb_pack(r: F32x8, g: F32x8, b: F32x8, a: F32x8) -> U32x8 {
    let zero = F32x8::splat(0.0);
    let one  = F32x8::splat(1.0);
    unsafe { argb_pack_clamped(
        r.clamp(zero, one),
        g.clamp(zero, one),
        b.clamp(zero, one),
        a.clamp(zero, one),
    ) }
}


pub const fn ts_per_unit<T: ImgType>() -> usize {
    core::mem::size_of::<U8x32>() / core::mem::size_of::<T>()
}



pub unsafe trait ImgType {}

unsafe impl ImgType for u8 {}
unsafe impl ImgType for u32 {}
unsafe impl ImgType for f32 {}
unsafe impl ImgType for U8x8 {}
unsafe impl ImgType for U8x32 {}
unsafe impl ImgType for U32x8 {}
unsafe impl ImgType for F32x8 {}

macro_rules! img_read_impl {
    () => {

        #[inline(always)]
        pub fn format(&self) -> ImageFormat { self.format }

        #[inline(always)]
        pub fn bounds(&self) -> U32x2 { self.bounds }

        #[inline(always)]
        pub fn width(&self) -> u32 { self.bounds[0] }

        #[inline(always)]
        pub fn height(&self) -> u32 { self.bounds[1] }

        #[inline(always)]
        pub fn unit_stride(&self) -> usize {
            self.stride as usize
        }

        #[inline(always)]
        pub fn unit_stride_bytes(&self) -> usize {
            std::mem::size_of::<U8x32>() * (self.stride as usize)
        }

        #[inline(always)]
        pub fn stride<T: ImgType>(&self) -> usize {
            ts_per_unit::<T>() * self.stride as usize
        }

        #[inline(always)]
        pub fn bounds_check_index<T: ImgType>(&self, index: usize) {
            let t_length = ts_per_unit::<T>() * self.data.len();
            if !(index < t_length) {
                unsafe { core::intrinsics::breakpoint() };
                panic!("image index bounds check failed ({}, {})", index, t_length);
            }
        }

        #[inline(always)]
        pub fn bounds_check_offset<T: ImgType>(&self, offset: usize) {
            let access_end = offset + core::mem::size_of::<T>();
            let buffer_end = self.data.len() * core::mem::size_of::<U8x32>();
            if !(access_end <= buffer_end) {
                unsafe { core::intrinsics::breakpoint() };
                panic!("image offset bounds check failed ({}, {})", access_end, buffer_end);
            }
        }


        #[inline(always)]
        pub unsafe fn read_unck<T: ImgType>(&self, index: usize) -> T {
            let base = self.data.as_ptr() as *const T;
            base.add(index).read()
        }

        #[inline(always)]
        pub fn read<T: ImgType>(&self, index: usize) -> T {
            self.bounds_check_index::<T>(index);
            unsafe { self.read_unck(index) }
        }

        #[inline(always)]
        pub fn read_xy<T: ImgType>(&self, x: usize, y: usize) -> T {
            self.read::<T>(self.stride::<T>()*y + x)
        }


        #[inline(always)]
        pub unsafe fn slice_unck<T: ImgType>(&self, begin: usize, end: usize) -> &[T] {
            let base = self.data.as_ptr() as *const T;
            core::slice::from_raw_parts(base.add(begin), end - begin)
        }

        #[inline(always)]
        pub fn slice<T: ImgType>(&self, begin: usize, end: usize) -> &[T] {
            assert!(begin <= end);
            assert!(end <= ts_per_unit::<T>() * self.data.len());
            unsafe { self.slice_unck(begin, end) }
        }


        #[inline(always)]
        pub unsafe fn read_offset_unck<T: ImgType>(&self, offset: usize) -> T {
            let base = self.data.as_ptr() as *const u8;
            (base.add(offset) as *const T).read_unaligned()
        }

        #[inline(always)]
        pub fn read_offset<T: ImgType>(&self, offset: usize) -> T {
            self.bounds_check_offset::<T>(offset);
            unsafe { self.read_offset_unck(offset) }
        }

        #[inline(always)]
        pub fn read_offset_xy<T: ImgType>(&self, x: usize, y: usize) -> T {
            self.read_offset::<T>(self.unit_stride_bytes()*y + x)
        }
    };
}

macro_rules! img_write_impl {
    () => {
        #[inline(always)]
        pub unsafe fn write_unck<T: ImgType>(&mut self, index: usize, value: T) {
            let base = self.data.as_mut_ptr() as *mut T;
            base.add(index).write(value);
        }

        #[inline(always)]
        pub fn write<T: ImgType>(&mut self, index: usize, value: T) {
            self.bounds_check_index::<T>(index);
            unsafe { self.write_unck(index, value) };
        }

        #[inline(always)]
        pub fn write_xy<T: ImgType>(&mut self, x: usize, y: usize, value: T) {
            self.write::<T>(self.stride::<T>()*y + x, value);
        }


        #[inline(always)]
        pub unsafe fn ref_mut_unck<T: ImgType>(&mut self, index: usize) -> &mut T {
            let base = self.data.as_mut_ptr() as *mut T;
            &mut *base.add(index)
        }

        #[inline(always)]
        pub fn ref_mut<T: ImgType>(&mut self, index: usize) -> &mut T {
            self.bounds_check_index::<T>(index);
            unsafe { self.ref_mut_unck(index) }
        }

        #[inline(always)]
        pub fn ref_mut_xy<T: ImgType>(&mut self, x: usize, y: usize) -> &mut T {
            self.ref_mut::<T>(self.stride::<T>()*y + x)
        }


        #[inline(always)]
        pub unsafe fn write_offset_unck<T: ImgType>(&mut self, offset: usize, value: T) {
            let base = self.data.as_mut_ptr() as *mut u8;
            (base.add(offset) as *mut T).write_unaligned(value);
        }

        #[inline(always)]
        pub fn write_offset<T: ImgType>(&mut self, offset: usize, value: T) {
            self.bounds_check_offset::<T>(offset);
            unsafe { self.write_unck(offset, value) };
        }

        #[inline(always)]
        pub fn write_offset_xy<T: ImgType>(&mut self, x: usize, y: usize, value: T) {
            self.write_offset::<T>(self.unit_stride_bytes()*y + x, value);
        }
    };
}



pub struct Image<A: Allocator = Global> {
    data:   Box<[U8x32], A>,
    format: ImageFormat,
    bounds: U32x2,
    stride: u32,
}

impl Image {
    pub fn new(format: ImageFormat, width: u32, height: u32) -> Image {
        Image::new_in(format, width, height, Global)
    }
}

impl<A: Allocator> Image<A> {
    pub fn new_in(format: ImageFormat, width: u32, height: u32, allocator: A) -> Image<A> {
        let (size, stride) = format.image_size(width, height);

        let data = {
            let mut data = Vec::new_in(allocator);
            data.resize(size as usize, Default::default());
            data.into_boxed_slice()
        };

        let bounds = [width, height].into();

        Image { data, format, bounds, stride }
    }
    

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


    pub fn img(&self) -> Img {
        Img {
            data:   &self.data,
            format: self.format, 
            bounds: self.bounds,
            stride: self.stride,
        }
    }

    pub fn img_mut(&mut self) -> ImgMut {
        ImgMut {
            data:   &mut self.data,
            format: self.format, 
            bounds: self.bounds,
            stride: self.stride,
        }
    }

    img_read_impl!();
    img_write_impl!();
}


pub struct Img<'a> {
    data:   &'a [U8x32],
    format: ImageFormat,
    bounds: U32x2,
    stride: u32,
}


impl<'a> Img<'a> {
    img_read_impl!();
}


pub struct ImgMut<'a> {
    data:   &'a mut [U8x32],
    format: ImageFormat,
    bounds: U32x2,
    stride: u32,
}

impl<'a> ImgMut<'a> {
    img_read_impl!();
    img_write_impl!();
}

