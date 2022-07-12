use sti::simd::*;

use crate::float::*;
use crate::alloc::*;
use crate::geometry::*;
use crate::path::*;
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

pub struct Rasterizer<A: Alloc> {
    pub flatten_tolerance: f32,
    pub flatten_recursion: u32,
    deltas: Image<f32, A>,
    size: F32x2,
    safe_size: F32x2,
    deltas_len: i32,
    buffer: [[F32x2; 2]; BUFFER_SIZE],
    buffered: usize,
}


impl Rasterizer<GlobalAlloc> {
    pub fn new(size: U32x2) -> Self {
        Rasterizer::new_in(size, GlobalAlloc)
    }
}

impl<A: Alloc> Rasterizer<A> {
    pub fn new_in(size: U32x2, alloc: A) -> Self {
        let mask_size = size + U32x2::new(2, 1);
        let deltas = Image::new_in(mask_size.x(), mask_size.y(), 0.0, alloc);
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

    /// - clip must be a valid integer rect with clip.min >= zero.
    /// - returns (raster_size, raster_origin, blit_offset).
    /// - raster_size is the size of the rasterizer's rect.
    /// - raster_origin is the global position of the rasterizer's origin.
    /// - blit_offset is the integer offset from aabb to the rasterizer's origin.
    pub fn rect_for(rect: Rect, clip: Rect, align: u32) -> (U32x2, F32x2, U32x2) {
        // compute rasterizer's integer rect in global coordinates.
        let raster_rect = unsafe { rect.clamp_to(clip).round_inclusive_unck() };

        // pad rasterizer rect to meet alignment requirement.
        let align = align as f32;
        let x0 = floor_fast(raster_rect.min.x() / align) * align;
        let x1 = ceil_fast(raster_rect.max.x() / align)  * align;

        let raster_size   = F32x2::new(x1 - x0, raster_rect.height()).to_i32().as_u32();
        let raster_origin = F32x2::new(x0,      raster_rect.min.y());
        let blit_offset   = (raster_origin - clip.min).to_i32().as_u32();
        (raster_size, raster_origin, blit_offset)
    }

    pub fn width(&self)  -> u32 { self.deltas.width() - 2 }
    pub fn height(&self) -> u32 { self.deltas.height() - 1 }


    #[cfg(not(target_arch = "x86_64"))]
    pub fn accumulate(mut self) -> Mask<'a> {
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

        deltas.truncate(w as u32, h as u32);
        deltas
    }

    #[cfg(target_arch = "x86_64")]
    pub fn accumulate(mut self) -> Image<f32, A> {
        if self.buffered > 0 {
            self.flush();
        }

        let w = self.width() as usize;
        let h = self.height() as usize;

        let mut deltas = self.deltas;

        for y in 0..h {

            let mut c = F32x4::ZERO;

            let aligned_w = w/4*4;

            for x in (0..aligned_w).step_by(4) {
                let mut d = deltas.read_n(x, y).into();

                d = d + shift::<4>(d);
                d = d + shift::<8>(d);
                c = c + d;

                deltas.write_n(x, y, c.abs().at_mostf(F32x4::splat(1.0)).into());

                c = F32x4::splat(c[3]);


                #[inline(always)]
                fn shift<const AMOUNT: i32>(v: F32x4) -> F32x4 { unsafe {
                    use core::arch::x86_64::*;
                    let m128 = _mm_slli_si128::<AMOUNT>(v.to_bits().into());
                    F32x4::from_bits(m128.into())
                }}
            }

            let mut c = c[3];
            for x in aligned_w..w {
                c += deltas[(x, y)];
                deltas[(x, y)] = c.abs().min(1.0);
            }
        }

        deltas.truncate(U32x2::new(w as u32, h as u32));
        deltas
    }


    #[inline(always)]
    pub unsafe fn add_segment_bounded(&mut self, p0: F32x2, p1: F32x2) {
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

        type F32v = F32x<WIDTH>;
        type I32v = I32x<WIDTH>;

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

                let (x0x1_0, y0y1_0) = x0y0x1y1_0.deinterleave(x0y0x1y1_1);
                let (x0x1_1, y0y1_1) = x0y0x1y1_2.deinterleave(x0y0x1y1_3);
                let (x0, x1) = x0x1_0.deinterleave(x0x1_1);
                let (y0, y1) = y0y1_0.deinterleave(y0y1_1);
                (x0, y0, x1, y1)
            };

            let stride = F32v::splat(self.deltas.stride() as f32);

            let dx = x1 - x0;
            let dy = y1 - y0;
            let dx_inv = safe_div(F32v::ONE, dx, F32v::splat(1_000_000.0));
            let dy_inv = safe_div(F32v::ONE, dy, F32v::splat(1_000_000.0));

            let x_step = F32v::ONE.copysign(dx);
            let y_step = F32v::ONE.copysign(dy);
            let x_nudge = -dx.lanes_lt(F32v::ZERO).as_i32().to_f32();
            let y_nudge = -dy.lanes_lt(F32v::ZERO).as_i32().to_f32();

            let x_dt = dx_inv.abs();
            let y_dt = dy_inv.abs();

            let x_i0 = unsafe { x0.to_i32_unck().to_f32() };
            let y_i0 = unsafe { y0.to_i32_unck().to_f32() };
            let x_i1 = unsafe { x1.to_i32_unck().to_f32() };
            let y_i1 = unsafe { y1.to_i32_unck().to_f32() };

            let x_steps = unsafe { (x_i1 - x_i0).abs().to_i32_unck() };
            let y_steps = unsafe { (y_i1 - y_i0).abs().to_i32_unck() };
            let steps = x_steps + y_steps;
            let max_steps = steps.reduce_max();

            let mut x_prev = x0;
            let mut y_prev = y0;
            let mut x_next = x_i0 + x_step + x_nudge;
            let mut y_next = y_i0 + y_step + y_nudge;
            let mut x_t_next = (x_next - x0) * dx_inv;
            let mut y_t_next = (y_next - y0) * dy_inv;
            let mut x_rem = x_steps;
            let mut y_rem = y_steps;

            let row_delta = unsafe { stride.copysign(dy).to_i32_unck() };
            let mut row_base = unsafe { (stride * y_i0).to_i32_unck() };
            let mut x_i = x_i0;

            for _ in 0..max_steps {
                let prev_base = row_base;
                let prev_x_i  = x_i;

                let x_left = x_rem.lanes_gt(I32v::ZERO);
                let y_left = y_rem.lanes_gt(I32v::ZERO);
                let any_left = x_left | y_left;
                let is_x = (x_t_next.lanes_le(y_t_next) & x_left) | !y_left;
                let is_y = !is_x;

                let x = any_left.select(is_x.select(x_next, x0 + y_t_next*dx), x_prev);
                let y = any_left.select(is_y.select(y_next, y0 + x_t_next*dy), y_prev);

                x_next   += is_x.select(x_step, F32v::ZERO);
                y_next   += is_y.select(y_step, F32v::ZERO);
                x_t_next += is_x.select(x_dt, F32v::ZERO);
                y_t_next += is_y.select(y_dt, F32v::ZERO);

                x_i      += (is_x & x_left).select(x_step, F32v::ZERO);
                x_rem    -= is_x.select(I32v::ONE, I32v::ZERO);

                row_base += (is_y & y_left).select(row_delta, I32v::ZERO);
                y_rem    -= is_y.select(I32v::ONE, I32v::ZERO);

                add_deltas(self, prev_base, prev_x_i, x_prev, y_prev, x, y);

                x_prev = x;
                y_prev = y;
            }

            debug_assert!(row_base.lanes_eq((stride * y_i1).to_i32()).all());
            debug_assert!(x_i.lanes_eq(x_i1).all());

            add_deltas(self, row_base, x_i, x_prev, y_prev, x1, y1);
        }

        self.buffered = 0;

        return;
        

        #[inline(always)]
        fn safe_div(a: F32v, b: F32v, default: F32v) -> F32v {
            let is_zero = b.lanes_eq(F32v::ZERO);
            is_zero.select(default, a/b)
        }

        #[inline(always)]
        fn add_deltas<A: Alloc>(r: &mut Rasterizer<A>, row_base: I32v, x_i: F32v,
            x0: F32v, y0: F32v, x1: F32v, y1: F32v
        ) {
            let delta = y1 - y0;

            let x_mid = (x0 + x1).div(2.0) - x_i;
            let delta_right = delta * x_mid;
            let delta_left  = delta - delta_right;

            debug_assert!(x_mid.lanes_ge(F32v::splat(0.0 - ZERO_TOLERANCE)).all());
            debug_assert!(x_mid.lanes_le(F32v::splat(1.0 + ZERO_TOLERANCE)).all());
            debug_assert!(x_i.lanes_ge(F32v::splat(0.0)).all());
            debug_assert!(x_i.lanes_le(F32v::splat(r.size.x())).all()); // le because padding

            let x = unsafe { x_i.to_i32_unck() };
            let o = row_base + x;

            assert!(o.lanes_ge(I32v::splat(0)).all());
            assert!(o.lanes_lt(I32v::splat(r.deltas_len - 1)).all());

            let deltas = r.deltas.data_mut().as_mut_ptr();
            for i in 0..WIDTH {
                unsafe {
                    *deltas.add(o[i] as usize + 0) += delta_left[i];
                    *deltas.add(o[i] as usize + 1) += delta_right[i];
                }
            }
        }

    }

    pub unsafe fn add_left_delta_bounded(&mut self, y0: f32, y1: f32) {
        debug_assert!(self.are_bounded(&[F32x2::new(0.0, y0), F32x2::new(0.0, y1)]));
        //println!("Segment(({}, {}), ({}, {})),", 0.0, y0, 0.0, y1);

        let stride = self.deltas.stride() as f32;

        let dy = y1 - y0;

        let y_step  = 1f32.copysign(dy);
        let y_nudge = if dy >= 0.0 { 0f32 } else { 1f32 };

        let y_i0 = floor_fast(y0);
        let y_i1 = floor_fast(y1);

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
        unsafe fn add_delta<A: Alloc>(r: &mut Rasterizer<A>, row_base: usize, y0: f32, y1: f32) {
            let delta = y1 - y0;
            r.deltas.data_mut()[row_base + 0] += delta;
        }
    }

    pub fn add_left_delta(&mut self, y0: f32, y1: f32) {
        let y0 = y0.clamp(0.0, self.size.y());
        let y1 = y1.clamp(0.0, self.size.y());
        unsafe { self.add_left_delta_bounded(y0, y1) }
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
        fn clamp<A: Alloc>(r: &mut Rasterizer<A>,
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

    #[inline(always)]
    pub fn add_segment(&mut self, segment: Segment) {
        self.add_segment_p(segment.p0, segment.p1)
    }


    pub fn add_quadratic_tol_rec(&mut self,
        quadratic: Quadratic,
        tolerance_squared: f32, max_recursion: u32
    ) {
        if self.is_invisible(quadratic.aabb()) {
            return;
        }

        if self.are_bounded(&[ quadratic.p0, quadratic.p1, quadratic.p2]) {
            let mut f = |p0, p1, _| {
                unsafe { self.add_segment_bounded(p0, p1) };
            };
            quadratic.flatten(&mut f, tolerance_squared, max_recursion);
        }
        else {
            let mut f = |p0, p1, _| {
                self.add_segment_p(p0, p1)
            };
            quadratic.flatten(&mut f, tolerance_squared, max_recursion);
        }
    }

    pub fn add_quadratic(&mut self, quadratic: Quadratic) {
        self.add_quadratic_tol_rec(
            quadratic,
            self.flatten_tolerance.squared(),
            self.flatten_recursion);
    }

    pub fn add_quadratic_p(&mut self, p0: F32x2, p1: F32x2, p2: F32x2) {
        self.add_quadratic(quadratic(p0, p1, p2))
    }


    pub fn add_cubic(&mut self, cubic: Cubic) {
        if self.is_invisible(cubic.aabb()) {
            return;
        }

        let tol = self.flatten_tolerance.squared();
        let rec = self.flatten_recursion;
        let mut f = |p0, p1, _| {
            self.add_segment_p(p0, p1);
        };
        cubic.flatten(&mut f, tol, rec);
    }

    pub fn add_cubic_p(&mut self, p0: F32x2, p1: F32x2, p2: F32x2, p3: F32x2) {
        self.add_cubic(cubic(p0, p1, p2, p3))
    }


    pub fn fill_path<B: Alloc>(&mut self, path: &Path<B>, tfx: Transform) {
        use IterEvent::*;
        let mut begin = None;

        for event in path.iter() {
            match event {
                Begin(p0, is_closed) => {
                    if !is_closed {
                        begin = Some(p0);
                    }
                }

                Segment(segment)     => { self.add_segment(tfx * segment); }
                Quadratic(quadratic) => { self.add_quadratic(tfx * quadratic); }
                Cubic(cubic)         => { self.add_cubic(tfx * cubic); }

                End (p1) => {
                    if let Some(p0) = begin {
                        self.add_segment(tfx * segment(p1, p0));
                        begin = None;
                    }
                }
            }
        }
    }

    pub fn fill_soa_path(&mut self, path: &SoaPath, tfx: Transform) {
        for line  in path.lines.iter()  { self.add_segment(tfx * *line); }
        for quad  in path.quads.iter()  { self.add_quadratic(tfx * *quad); }
        for cubic in path.cubics.iter() { self.add_cubic(tfx * *cubic); }
    }


    #[inline(always)]
    pub fn is_invisible(&self, aabb: Rect) -> bool {
        aabb.min.lanes_ge(self.size).any() || aabb.max.y() <= 0.0
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
}

