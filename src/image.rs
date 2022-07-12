use core::marker::PhantomData;

use sti::simd::*;
use crate::alloc::*;
use crate::geometry::rect;
use crate::renderer::Tile;



pub struct Image<T: Copy, A: Alloc = GlobalAlloc> {
    data: Box<[T], A>,
    size: U32x2,
}

impl<T: Copy> Image<T, GlobalAlloc> {
    pub unsafe fn new_uninit(w: u32, h: u32) -> Self {
        Image::new_uninit_in(w, h, GlobalAlloc)
    }

    pub fn new(w: u32, h: u32, clear: T) -> Self {
        Image::new_in(w, h, clear, GlobalAlloc)
    }
}

impl<T: Copy, A: Alloc> Image<T, A> {
    #[inline(always)]
    pub fn data(&self) -> &[T] {
        &self.data
    }

    #[inline(always)]
    pub fn data_mut(&mut self) -> &mut [T] {
        &mut self.data
    }

    #[inline(always)]
    pub fn size(&self) -> U32x2 {
        self.size
    }

    #[inline(always)]
    pub fn stride(&self) -> usize {
        self.size.x() as usize
    }

    #[inline(always)]
    pub fn stride_bytes(&self) -> usize {
        self.stride() * core::mem::size_of::<T>()
    }


    pub unsafe fn new_uninit_in(w: u32, h: u32, alloc: A) -> Self {
        let len = (w * h) as usize;

        let mut data = Vec::with_capacity_in(len, alloc);
        data.set_len(len);

        Self { data: data.into_boxed_slice(), size: U32x2::new(w, h) }
    }

    pub fn new_in(w: u32, h: u32, clear: T, alloc: A) -> Self {
        let mut result = unsafe { Self::new_uninit_in(w, h, alloc) };
        result.clear(clear);
        result
    }

    pub fn clear(&mut self, color: T) {
        for v in self.data.iter_mut() {
            *v = color;
        }
    }


    unsafe fn _view<const MUT: bool>(&self) -> BaseImg<MUT, T> {
        BaseImg {
            data: (self.data.as_ptr(), self.data.len(), PhantomData),
            size: self.size,
            stride: self.stride(),
        }
    }

    #[inline(always)]
    pub fn view(&self) -> Img<T> {
        unsafe { self._view() }
    }

    #[inline(always)]
    pub fn view_mut(&mut self) -> ImgMut<T> {
        unsafe { self._view() }
    }
}


pub struct BaseImg<'img, const MUT: bool, T: Copy> {
    data:   (*const T, usize, PhantomData<&'img ()>),
    size:   U32x2,
    stride: usize,
}

pub type Img<'img, T> = BaseImg<'img, false, T>;
pub type ImgMut<'img, T> = BaseImg<'img, true, T>;


impl<'img, const MUT: bool, T: Copy> BaseImg<'img, MUT, T> {
    #[inline(always)]
    pub fn data(&self) -> &[T] {
        unsafe { core::slice::from_raw_parts(self.data.0, self.data.1) }
    }

    #[inline(always)]
    pub fn size(&self) -> U32x2 {
        self.size
    }

    #[inline(always)]
    pub fn stride(&self) -> usize {
        self.stride
    }

    #[inline(always)]
    pub fn stride_bytes(&self) -> usize {
        self.stride() * core::mem::size_of::<T>()
    }
}

impl<'img, T: Copy> ImgMut<'img, T> {
    #[inline(always)]
    pub fn data_mut(&mut self) -> &mut [T] {
        unsafe { core::slice::from_raw_parts_mut(self.data.0 as *mut _, self.data.1) }
    }

    #[inline(always)]
    pub fn view(&self) -> Img<T> {
        Img { data: self.data, size: self.size, stride: self.stride }
    }


    pub fn clear(&mut self, color: T) {
        for v in self.data_mut().iter_mut() {
            *v = color;
        }
    }

    pub fn copy_expand<U: Copy, const N: usize, F: Fn(U) -> [T; N]>
        (&mut self, src: &Img<U>, to: I32x2, f: F)
    {
        let size = src.size.as_i32()*I32x2::new(N as i32, 1);

        let begin = to         .clamp(I32x2::ZERO, self.size.as_i32()).cast::<usize>();
        let end   = (to + size).clamp(I32x2::ZERO, self.size.as_i32()).cast::<usize>();

        let [w, h] = *(end - begin).as_array();

        let stride = self.stride;
        let data = self.data_mut();
        let start = begin.y()*stride + begin.x();

        for dy in 0..h {
            let base = start + dy*stride;

            for u in 0 .. (w / N) {
                let c = f(src[(u, dy)]);
                let i0 = base + u*N;
                data[i0 .. i0 + N].copy_from_slice(&c);
            }

            let rem = w % N;
            if rem > 0 {
                let u = w / N;
                let c = f(src[(u, dy)]);
                let i0 = base + u*N;
                data[i0 .. i0 + rem].copy_from_slice(&c[0..rem]);
            }
        }
    }


    pub fn sub_view(&mut self, begin: U32x2, end: U32x2) -> ImgMut<T> {
        assert!(begin.lanes_le(end).all());
        assert!(end.lanes_le(self.size).all());
        let size = end - begin;

        let index = (begin.y() as usize)*self.stride + begin.x() as usize;

        let mut len = 0;
        if size.y() > 0 {
            len += size.x() as usize;
            len += (size.y() as usize - 1)*self.stride;
        }
        assert!(index + len <= self.data.1);

        let ptr = unsafe { self.data.0.add(index) }; 
        ImgMut {
            data: (ptr, len, PhantomData),
            size,
            stride: self.stride,
        }
    }

    #[inline(always)]
    pub fn sub_tile(&mut self, begin: U32x2, end: U32x2) -> Tile<T> {
        Tile {
            img: self.sub_view(begin, end),
            rect: rect(begin.as_i32().to_f32(), end.as_i32().to_f32()),
        }
    }

    pub fn tiles(&mut self, tile_size: u32) -> (Vec<Tile<T>>, U32x2) {
        let tile_size = U32x2::splat(tile_size);
        let tile_counts = (self.size + tile_size - U32x2::ONE) / tile_size;
        let tile_count = (tile_counts.x() * tile_counts.y()) as usize;

        let mut tiles = Vec::with_capacity(tile_count);
        for y in 0..tile_counts.y() {
            for x in 0..tile_counts.x() {
                let begin = U32x2::new(x, y) * tile_size;
                let end   = (begin + tile_size).min(self.size);

                let tile = self.sub_tile(begin, end);
                // cast lifetime to outer borrow.
                // tiles are disjoint by construction.
                let tile = unsafe { core::mem::transmute(tile) };
                tiles.push(tile);
            }
        }

        (tiles, tile_counts)
    }
}


impl<'img, const MUT: bool, T: Copy> core::ops::Index<(usize, usize)> for BaseImg<'img, MUT, T> {
    type Output = T;

    #[inline(always)]
    fn index(&self, index: (usize, usize)) -> &Self::Output {
        let (x, y) = index;
        assert!(x < self.size.x() as usize);
        let stride = self.stride;
        // y bounds check is here.
        &self.data()[y*stride + x]
    }
}

impl<'img, T: Copy> core::ops::IndexMut<(usize, usize)> for BaseImg<'img, true, T> {
    #[inline(always)]
    fn index_mut(&mut self, index: (usize, usize)) -> &mut Self::Output {
        let (x, y) = index;
        assert!(x < self.size.x() as usize);
        let stride = self.stride;
        // y bounds check is here.
        &mut self.data_mut()[y*stride + x]
    }
}




pub type Mask<'a>   = Image_a_f32<'a>;



fn new_slice_box<T: Default + Copy>(length: usize, allocator: &dyn Alloc) -> Box<[T], &dyn Alloc> {
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
    pub(crate) data: Box<[f32], &'a dyn Alloc>,
    bounds: U32x2,
    stride: usize,
}

impl<'a> Image_a_f32<'a> {
    pub fn new(width: u32, height: u32) -> Image_a_f32<'a> {
        Image_a_f32::new_in(width, height, &GlobalAlloc)
    }

    pub fn new_in(width: u32, height: u32, allocator: &'a dyn Alloc) -> Image_a_f32<'a> {
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




#[inline(always)]
pub fn argb_unpack(v: u32) -> F32x4 {
    let a = (v >> 24) & 0xff;
    let r = (v >> 16) & 0xff;
    let g = (v >>  8) & 0xff;
    let b = (v >>  0) & 0xff;

    let color = U32x4::new(r, g, b, a);
    let scale = F32x4::splat(255.0);
    color.as_i32().to_f32() / scale
}

#[inline(always)]
pub fn argb_pack_u8s(r: u8, g: u8, b: u8, a: u8) -> u32 {
    let (r, g, b, a) = (r as u32, g as u32, b as u32, a as u32);
    a << 24 | r << 16 | g << 8 | b
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

