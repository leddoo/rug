extern crate alloc;
use alloc::alloc::*;

use crate::float::Float;
use crate::geometry::*;
use crate::image::*;


// these are absolute and in pixel space.
pub const ZERO_TOLERANCE:    f32 = 0.001;
pub const FLATTEN_TOLERANCE: f32 = 0.1;
pub const FLATTEN_RECURSION: u32 = 16;


pub struct Rasterizer<A: Allocator = Global> {
    deltas: ImageF32<1, A>,
    pub flatten_tolerance: f32,
    pub flatten_recursion: u32,
}


impl Rasterizer {
    pub fn new(width: u32, height: u32) -> Rasterizer {
        Rasterizer::new_in(width, height, Global)
    }
}

impl<A: Allocator> Rasterizer<A> {
    pub fn new_in(width: u32, height: u32, allocator: A) -> Rasterizer<A> {
        Rasterizer {
            deltas: ImageF32::new_in(width + 1, height + 1, allocator),
            flatten_tolerance: FLATTEN_TOLERANCE,
            flatten_recursion: FLATTEN_RECURSION,
        }
    }

    pub fn width(&self)  -> u32 { self.deltas.width() - 1 }
    pub fn height(&self) -> u32 { self.deltas.height() - 1 }

    pub fn accumulate(self) -> ImageF32<1, A> {
        let w = self.width();
        let h = self.height();

        let mut deltas = self.deltas;
        let stride = deltas.stride();

        for y in 0..h {
            let mut a = 0.0;
            for x in 0..w {
                a += deltas.raw_index(y*stride + x);
                *deltas.raw_index_mut(y*stride + x) = a.abs().min(1.0);
            }
        }

        deltas.truncate(w, h);
        deltas
    }


    fn add_delta(&mut self, row_base: u32, x_i: f32, x0: f32, y0: f32, x1: f32, y1: f32) {
        let delta = y1 - y0;

        if x_i < 0.0 {
            *self.deltas.raw_index_mut(row_base) += delta;
        }
        else if x_i < self.width() as f32 {
            let x_mid = (x0 + x1) / 2.0 - x_i;
            let delta_right = delta * x_mid;
            debug_assert!(x_mid >= 0.0 && x_mid <= 1.0);

            let x = x_i as u32;
            *self.deltas.raw_index_mut(row_base + x + 0) += delta - delta_right;
            *self.deltas.raw_index_mut(row_base + x + 1) += delta_right;
        }
    }


    pub fn add_segment_p(&mut self, p0: V2f, p1: V2f) {
        let stride = self.deltas.stride() as f32;
        let height = self.height() as f32;

        let dx_over_dy = (p1.x - p0.x).safe_div(p1.y - p0.y, 0.0);
        let (x0, y0) = clamp_y(p0.x, p0.y, dx_over_dy, height);
        let (x1, y1) = clamp_y(p1.x, p1.y, dx_over_dy, height);

        let dx = x1 - x0;
        let dy = y1 - y0;
        let dx_inv = 1.0.safe_div(dx, 1_000_000.0);
        let dy_inv = 1.0.safe_div(dy, 1_000_000.0);

        let x_step = 1f32.copysign(dx);
        let y_step = 1f32.copysign(dy);
        let x_nudge = if dx >= 0.0 { 0f32 } else { 1f32 };
        let y_nudge = if dy >= 0.0 { 0f32 } else { 1f32 };

        let x_dt = dx_inv.abs();
        let y_dt = dy_inv.abs();

        let x_i0 = x0.floor();
        let y_i0 = y0.floor();
        let x_i1 = x1.floor();
        let y_i1 = y1.floor();

        let x_steps = (x_i1 - x_i0).abs() as u32;
        let y_steps = (y_i1 - y_i0).abs() as u32;
        let steps = x_steps + y_steps;

        let mut x_prev = x0;
        let mut y_prev = y0;
        let mut x_next = x_i0 + x_step + x_nudge;
        let mut y_next = y_i0 + y_step + y_nudge;
        let mut x_t_next = (x_next - x0) * dx_inv;
        let mut y_t_next = (y_next - y0) * dy_inv;
        let mut x_rem = x_steps;
        let mut y_rem = y_steps;

        let     row_delta = stride.copysign(dy) as i32;
        let mut row_base  = (stride * y_i0) as i32;
        let mut x_i = x_i0;

        for _ in 0..steps {
            let prev_base = row_base;
            let prev_x_i  = x_i;
            let x;
            let y;
            if (x_t_next <= y_t_next && x_rem > 0) || y_rem == 0 {
                x = x_next;
                y = y0 + x_t_next*dy;

                x_next   += x_step;
                x_t_next += x_dt;
                x_i      += x_step;
                x_rem    -= 1;
            }
            else {
                x = x0 + y_t_next*dx;
                y = y_next;

                y_next   += y_step;
                y_t_next += y_dt;
                row_base += row_delta;
                y_rem    -= 1;
            }

            //println!("Segment(({}, {}), ({}, {})),", x_prev, y_prev, x, y);
            self.add_delta(prev_base as u32, prev_x_i, x_prev, y_prev, x, y);

            x_prev = x;
            y_prev = y;
        }

        debug_assert!(x_rem == 0);
        debug_assert!(row_base == (stride * y_i1) as i32);
        debug_assert!(x_i == x_i1);

        //println!("Segment(({}, {}), ({}, {})),", x_prev, y_prev, x1, y1);
        self.add_delta(row_base as u32, x_i, x_prev, y_prev, x1, y1);
    }

    pub fn add_segment(&mut self, segment: Segment) {
        self.add_segment_p(segment.p0, segment.p1)
    }


    pub fn add_quadratic(&mut self, quadratic: Quadratic) {
        let tol = self.flatten_tolerance.squared();
        let rec = self.flatten_recursion;
        let mut f = |p0, p1, _| {
            self.add_segment_p(p0, p1);
        };
        quadratic.flatten(&mut f, tol, rec);
    }

    pub fn add_quadratic_p(&mut self, p0: V2f, p1: V2f, p2: V2f) {
        self.add_quadratic(quadratic(p0, p1, p2))
    }


    pub fn add_cubic(&mut self, cubic: Cubic) {
        let tol = self.flatten_tolerance.squared();
        let rec = self.flatten_recursion;
        let mut f = |p0, p1, _| {
            self.add_segment_p(p0, p1);
        };
        cubic.flatten(&mut f, tol, rec);
    }

    pub fn add_cubic_p(&mut self, p0: V2f, p1: V2f, p2: V2f, p3: V2f) {
        self.add_cubic(cubic(p0, p1, p2, p3))
    }
}


fn clamp_y(x: f32, y: f32, dx_over_dy: f32, h: f32) -> (f32, f32) {
    if y < 0.0 {
        return (x + dx_over_dy*(0.0 - y), 0.0);
    }
    if y > h {
        return (x + dx_over_dy*(h - y), h);
    }
    (x, y)
}
