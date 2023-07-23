use sti::alloc::*;
use sti::simd::*;
use sti::float::F32Ext;

use crate::geometry::*;
use crate::image::*;


#[derive(Clone, Copy, Debug)]
pub enum FillRule {
    NonZero,
    EvenOdd,
}


// these are absolute and in pixel space.
pub const ZERO_TOLERANCE:    f32 = 0.001;
pub const ZERO_TOLERANCE_SQ: f32 = ZERO_TOLERANCE*ZERO_TOLERANCE;
pub const FLATTEN_TOLERANCE: f32 = 0.1;
pub const FLATTEN_RECURSION: u32 = 16;


const BUFFER_SIZE: usize = 32;

pub struct Rasterizer<'a> {
    pub flatten_tolerance: f32,
    pub flatten_recursion: u32,
    deltas: ImgMut<'a, f32>,
    size: F32x2,
    safe_size: F32x2,
    deltas_len: i32,
    buffer: [[F32x2; 2]; BUFFER_SIZE],
    buffered: usize,
}


impl<'a> Rasterizer<'a> {
    pub fn new<A: Alloc>(image: &'a mut Image<f32, A>, size: [u32; 2]) -> Self {
        let size = U32x2::from_array(size);
        let mask_size = size + U32x2::new(2, 1);
        image.resize_and_clear(*mask_size, 0.0);

        let deltas = image.view_mut();
        let deltas_len = deltas.data().len().try_into().unwrap();

        let size = size.as_i32().to_f32();
        Rasterizer {
            flatten_tolerance: FLATTEN_TOLERANCE,
            flatten_recursion: FLATTEN_RECURSION,
            deltas,
            size,
            safe_size: size + F32x2::splat(0.9),
            deltas_len,
            buffer: [[F32x2::ZERO; 2]; BUFFER_SIZE],
            buffered: 0,
        }
    }

    pub fn width(&self)  -> u32 { self.deltas.width() - 2 }
    pub fn height(&self) -> u32 { self.deltas.height() - 1 }


    #[inline(always)]
    pub fn is_invisible(&self, aabb: Rect) -> bool {
        aabb.min.ge(self.size).any() || aabb.max.y() <= 0.0
    }

    #[inline(always)]
    pub fn is_bounded(&self, p0: F32x2) -> bool {
        let safe_rect = rect(F32x2::ZERO, self.safe_size);
        safe_rect.contains(p0)
    }

    #[inline(always)]
    pub fn are_bounded(&self, ps: &[F32x2]) -> bool {
        ps.iter().all(|p| self.is_bounded(*p))
    }


    #[inline(always)]
    pub fn add_segment_p(&mut self, p0: F32x2, p1: F32x2) {
        let aabb = Rect::from_points(p0, p1);
        if self.is_invisible(aabb) {
            return;
        }

        if self.are_bounded(&[p0, p1]) {
            unsafe { self.add_segment_bounded(p0, p1); }
        }
        else {
            self._add_segment_p_slow_path(p0, p1)
        }
    }


    #[inline(always)]
    pub fn add_segment(&mut self, segment: Segment) {
        self.add_segment_p(segment.p0, segment.p1)
    }


    pub fn accumulate(mut self) -> ImgMut<'a, f32> {
        if self.buffered > 0 {
            self.flush();
        }

        let w = self.width() as usize;
        let h = self.height() as usize;

        let mut deltas = self.deltas;

        for y in 0..h {
            let mut c = 0.0;
            for x in 0..w {
                c += deltas[(x, y)];
                deltas[(x, y)] = c.abs().min(1.0);
            }
        }

        deltas.truncate([w as u32, h as u32]);
        deltas
    }

}


impl<'a> Rasterizer<'a> {
    /// - clip must be a valid integer rect with clip.min >= zero.
    /// - returns (raster_size, raster_origin, blit_offset).
    /// - raster_size is the size of the rasterizer's rect.
    /// - raster_origin is the global position of the rasterizer's origin.
    /// - blit_offset is the integer offset from aabb to the rasterizer's origin.
    fn rect_for(rect: Rect, clip: Rect, align: u32) -> (U32x2, F32x2, U32x2) {
        // compute rasterizer's integer rect in global coordinates.
        let raster_rect = rect.clamp_to(clip).round_inclusive_unck();

        // pad rasterizer rect to meet alignment requirement.
        let align = align as f32;
        let x0 = (raster_rect.min.x() / align).ffloor() * align;
        let x1 = (raster_rect.max.x() / align).fceil()  * align;

        let raster_size   = F32x2::new(x1 - x0, raster_rect.height()).to_i32_unck().as_u32();
        let raster_origin = F32x2::new(x0,      raster_rect.min.y());
        let blit_offset   = (raster_origin - clip.min).to_i32_unck().as_u32();
        (raster_size, raster_origin, blit_offset)
    }

    #[inline(always)]
    unsafe fn add_segment_bounded(&mut self, p0: F32x2, p1: F32x2) {
        debug_assert!(self.are_bounded(&[p0, p1]));
        //println!("Segment(({}, {}), ({}, {})),", p0.x(), p0.y(), p1.x(), p1.y());
        //println!("Vector(({}, {}), ({}, {})),", p0.x(), p0.y(), p1.x(), p1.y());

        if self.buffered >= self.buffer.len() {
            self.flush();
        }
        self.buffer[self.buffered][0] = p0;
        self.buffer[self.buffered][1] = p1;
        self.buffered += 1;
    }

    fn flush(&mut self) {
        const WIDTH: usize = 4;
        type F32v = F32x4;
        type I32v = I32x4;

        let batches = (self.buffered + WIDTH-1) / WIDTH;
        assert!(4*batches*core::mem::size_of::<F32v>() <= core::mem::size_of_val(&self.buffer));

        // zero out the extra segments, so we don't rasterize garbage.
        for i in self.buffered .. batches * WIDTH {
            self.buffer[i] = [F32x2::ZERO; 2]
        }

        for batch in 0..batches {
            let (x0, y0, x1, y1) = {
                let buffer = self.buffer.as_ptr() as *const F32v;

                let x0y0x1y1_0 = unsafe { buffer.add(4*batch + 0).read_unaligned() };
                let x0y0x1y1_1 = unsafe { buffer.add(4*batch + 1).read_unaligned() };
                let x0y0x1y1_2 = unsafe { buffer.add(4*batch + 2).read_unaligned() };
                let x0y0x1y1_3 = unsafe { buffer.add(4*batch + 3).read_unaligned() };

                let (x0x1_0, y0y1_0) = x0y0x1y1_0.unzip(x0y0x1y1_1);
                let (x0x1_1, y0y1_1) = x0y0x1y1_2.unzip(x0y0x1y1_3);
                let (x0, x1) = x0x1_0.unzip(x0x1_1);
                let (y0, y1) = y0y1_0.unzip(y0y1_1);
                (x0, y0, x1, y1)
            };

            let stride = F32v::splat(self.deltas.stride() as f32);

            let dx = x1 - x0;
            let dy = y1 - y0;
            let dx_inv = safe_div(F32v::ONE, dx, F32v::splat(1_000_000.0));
            let dy_inv = safe_div(F32v::ONE, dy, F32v::splat(1_000_000.0));

            let x_step = F32v::ONE.with_sign_of(dx);
            let y_step = F32v::ONE.with_sign_of(dy);
            let x_nudge = -dx.lt(F32v::ZERO).as_i32().to_f32();
            let y_nudge = -dy.lt(F32v::ZERO).as_i32().to_f32();

            let x_dt = dx_inv.abs();
            let y_dt = dy_inv.abs();

            let x_i0 = x0.to_i32_unck().to_f32();
            let y_i0 = y0.to_i32_unck().to_f32();
            let x_i1 = x1.to_i32_unck().to_f32();
            let y_i1 = y1.to_i32_unck().to_f32();

            let x_steps = (x_i1 - x_i0).abs().to_i32_unck();
            let y_steps = (y_i1 - y_i0).abs().to_i32_unck();
            let steps = x_steps + y_steps;
            let max_steps = steps.hmax();

            let mut x_prev = x0;
            let mut y_prev = y0;
            let mut x_next = x_i0 + x_step + x_nudge;
            let mut y_next = y_i0 + y_step + y_nudge;
            let mut x_t_next = (x_next - x0) * dx_inv;
            let mut y_t_next = (y_next - y0) * dy_inv;
            let mut x_rem = x_steps;
            let mut y_rem = y_steps;

            let row_delta = stride.with_sign_of(dy).to_i32_unck();
            let mut row_base = (stride * y_i0).to_i32_unck();
            let mut x_i = x_i0;

            for _ in 0..max_steps {
                let prev_base = row_base;
                let prev_x_i  = x_i;

                let x_left = x_rem.gt(I32v::ZERO);
                let y_left = y_rem.gt(I32v::ZERO);
                let any_left = x_left | y_left;
                let is_x = (x_t_next.le(y_t_next) & x_left) | !y_left;
                let is_y = !is_x;

                let x = any_left.select_f32(is_x.select_f32(x_next, x0 + y_t_next*dx), x_prev);
                let y = any_left.select_f32(is_y.select_f32(y_next, y0 + x_t_next*dy), y_prev);

                x_next   += is_x.select_f32(x_step, F32v::ZERO);
                y_next   += is_y.select_f32(y_step, F32v::ZERO);
                x_t_next += is_x.select_f32(x_dt, F32v::ZERO);
                y_t_next += is_y.select_f32(y_dt, F32v::ZERO);

                x_i      += (is_x & x_left).select_f32(x_step, F32v::ZERO);
                x_rem    -= is_x.select_i32(I32v::ONE, I32v::ZERO);

                row_base += (is_y & y_left).select_i32(row_delta, I32v::ZERO);
                y_rem    -= is_y.select_i32(I32v::ONE, I32v::ZERO);

                add_deltas(self, prev_base, prev_x_i, x_prev, y_prev, x, y);

                x_prev = x;
                y_prev = y;
            }

            debug_assert!(row_base.eq((stride * y_i1).to_i32_unck()).all());
            debug_assert!(x_i.eq(x_i1).all());

            add_deltas(self, row_base, x_i, x_prev, y_prev, x1, y1);
        }

        self.buffered = 0;

        return;
        

        #[inline(always)]
        fn safe_div(a: F32v, b: F32v, default: F32v) -> F32v {
            let is_zero = b.eq(F32v::ZERO);
            is_zero.select_f32(default, a/b)
        }

        #[inline(always)]
        fn add_deltas(r: &mut Rasterizer, row_base: I32v, x_i: F32v,
            x0: F32v, y0: F32v, x1: F32v, y1: F32v
        ) {
            let delta = y1 - y0;

            let x_mid = (x0 + x1)/2.0 - x_i;
            let delta_right = delta * x_mid;
            let delta_left  = delta - delta_right;

            debug_assert!(x_mid.ge(F32v::splat(0.0 - ZERO_TOLERANCE)).all());
            debug_assert!(x_mid.le(F32v::splat(1.0 + ZERO_TOLERANCE)).all());
            debug_assert!(x_i.ge(F32v::splat(0.0)).all());
            debug_assert!(x_i.le(F32v::splat(r.size.x())).all()); // le because padding

            let x = x_i.to_i32_unck();
            let o = row_base + x;

            assert!(o.ge(I32v::splat(0)).all());
            assert!(o.lt(I32v::splat(r.deltas_len - 1)).all());

            let deltas = r.deltas.data_mut().as_mut_ptr();
            for i in 0..WIDTH {
                unsafe {
                    *deltas.add(o[i] as usize + 0) += delta_left[i];
                    *deltas.add(o[i] as usize + 1) += delta_right[i];
                }
            }
        }

    }

    unsafe fn add_left_delta_bounded(&mut self, y0: f32, y1: f32) {
        debug_assert!(self.are_bounded(&[F32x2::new(0.0, y0), F32x2::new(0.0, y1)]));
        //println!("Segment(({}, {}), ({}, {})),", 0.0, y0, 0.0, y1);

        let stride = self.deltas.stride() as f32;

        let dy = y1 - y0;

        let y_step  = 1f32.copysign(dy);
        let y_nudge = if dy >= 0.0 { 0f32 } else { 1f32 };

        let y_i0 = y0.ffloor();
        let y_i1 = y1.ffloor();

        let y_steps = (y_i1 - y_i0).abs() as u32;
        let steps = y_steps;

        let mut y_prev = y0;
        let mut y_next = y_i0 + y_step + y_nudge;

        let     row_delta = stride.copysign(dy) as i32;
        let mut row_base  = (stride * y_i0) as i32;

        for _ in 0..steps {
            let prev_base = row_base;
            let y = y_next;
            y_next   += y_step;
            row_base += row_delta;

            //println!("Segment(({}, {}), ({}, {})),", x_prev, y_prev, x, y);
            add_delta(self, prev_base as isize as usize, y_prev, y);

            y_prev = y;
        }

        debug_assert!(row_base == (stride * y_i1) as i32);

        //println!("Segment(({}, {}), ({}, {})),", x_prev, y_prev, x1, y1);
        add_delta(self, row_base as isize as usize, y_prev, y1);


        #[inline(always)]
        unsafe fn add_delta(r: &mut Rasterizer, row_base: usize, y0: f32, y1: f32) {
            let delta = y1 - y0;
            r.deltas.data_mut()[row_base + 0] += delta;
        }
    }

    fn add_left_delta(&mut self, y0: f32, y1: f32) {
        let y0 = y0.clamp(0.0, self.size.y());
        let y1 = y1.clamp(0.0, self.size.y());
        unsafe { self.add_left_delta_bounded(y0, y1) }
    }

    #[inline(never)]
    fn _add_segment_p_slow_path(&mut self, p0: F32x2, p1: F32x2) {
        let (x0, y0) = (p0.x(), p0.y());
        let (x1, y1) = (p1.x(), p1.y());

        if x0 <= 0.0 + ZERO_TOLERANCE && x1 <= 0.0 + ZERO_TOLERANCE {
            self.add_left_delta(y0, y1);
            return;
        }

        let dx = (p1 - p0).x();
        let dy = (p1 - p0).y();
        let dx_over_dy = dx.safe_div(dy, 0.0);
        let dy_over_dx = dy.safe_div(dx, 0.0);

        let (x0, y0) = clamp(self, x0, y0, dx_over_dy, dy_over_dx, true);
        let (x1, y1) = clamp(self, x1, y1, dx_over_dy, dy_over_dx, false);

        unsafe { self.add_segment_bounded(F32x2::new(x0, y0), F32x2::new(x1, y1)); }


        #[inline(always)]
        fn clamp(r: &mut Rasterizer,
            mut x: f32, mut y: f32,
            dx_over_dy: f32, dy_over_dx: f32,
            is_first: bool)
            -> (f32, f32)
        {
            let w = r.size.x();
            let h = r.size.y();

            if y < 0.0 {
                x += dx_over_dy*(0.0 - y);
                y  = 0.0;
            }
            else if y > h {
                x += dx_over_dy*(h - y);
                y  = h;
            }

            if x < 0.0 {
                let y0 = y;

                y = (y + dy_over_dx*(0.0 - x)).clamp(0.0, h);
                x = 0.0;

                let y1 = y;
                if is_first {
                    unsafe { r.add_left_delta_bounded(y0, y1); }
                }
                else {
                    unsafe { r.add_left_delta_bounded(y1, y0); }
                }
            }
            else if x > w {
                y = (y + dy_over_dx*(w - x)).clamp(0.0, h);
                x = w;
            }

            (x, y)
        }
    }
}

