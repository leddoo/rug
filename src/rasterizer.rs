extern crate alloc;
use alloc::alloc::*;

use crate::float::*;
use crate::wide::*;
use crate::geometry::*;
use crate::path::*;
use crate::image::{Mask};


// these are absolute and in pixel space.
pub const ZERO_TOLERANCE:    f32 = 0.001;
pub const FLATTEN_TOLERANCE: f32 = 0.1;
pub const FLATTEN_RECURSION: u32 = 16;


pub struct Rasterizer<'a> {
    pub deltas: Mask<'a>,
    pub flatten_tolerance: f32,
    pub flatten_recursion: u32,
}


impl<'a> Rasterizer<'a> {
    pub fn new(width: u32, height: u32) -> Rasterizer<'a> {
        Rasterizer::new_in(width, height, &Global)
    }

    pub fn new_in(width: u32, height: u32, allocator: &'a dyn Allocator) -> Rasterizer<'a> {
        Rasterizer {
            deltas: Mask::new_in(width + 1, height + 1, allocator),
            flatten_tolerance: FLATTEN_TOLERANCE,
            flatten_recursion: FLATTEN_RECURSION,
        }
    }

    pub fn width(&self)  -> u32 { self.deltas.width() - 1 }
    pub fn height(&self) -> u32 { self.deltas.height() - 1 }

    pub fn accumulate(self) -> Mask<'a> {
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


    fn add_delta(&mut self, row_base: u32, x_i: f32, x0: f32, y0: f32, x1: f32, y1: f32) {
        let row_base = row_base as usize;
        let delta = y1 - y0;

        if x_i < 0.0 {
            self.deltas[row_base] += delta;
        }
        else if x_i < self.width() as f32 {
            let x_mid = (x0 + x1) / 2.0 - x_i;
            let delta_right = delta * x_mid;
            debug_assert!(x_mid >= 0.0 - ZERO_TOLERANCE && x_mid <= 1.0 + ZERO_TOLERANCE);

            let x = x_i as usize;
            self.deltas[row_base + x + 0] += delta - delta_right;
            self.deltas[row_base + x + 1] += delta_right;
        }
    }


    pub fn add_segment_p(&mut self, p0: F32x2, p1: F32x2) {
        let stride = self.deltas.stride() as f32;
        let height = self.height() as f32;

        let dx_over_dy = (p1.x() - p0.x()).safe_div(p1.y() - p0.y(), 0.0);
        let (x0, y0) = clamp_y(p0.x(), p0.y(), dx_over_dy, height);
        let (x1, y1) = clamp_y(p1.x(), p1.y(), dx_over_dy, height);

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

        let x_i0 = floor_fast(x0);
        let y_i0 = floor_fast(y0);
        let x_i1 = floor_fast(x1);
        let y_i1 = floor_fast(y1);

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


    pub fn add_quadratic_tol_rec(&mut self,
        quadratic: Quadratic,
        tolerance_squared: f32, max_recursion: u32
    ) {
        let mut f = |p0, p1, _| {
            self.add_segment_p(p0, p1);
        };
        quadratic.flatten(&mut f, tolerance_squared, max_recursion);
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


    pub fn fill_path(&mut self, path: &Path, position: F32x2) {
        for event in path.iter() {
            use IterEvent::*;
            match event {
                Segment(segment)     => { self.add_segment(segment + -position); },
                Quadratic(quadratic) => { self.add_quadratic(quadratic + -position); },
                Cubic(cubic)         => { self.add_cubic(cubic + -position); },
                _ => (),
            }
        }
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
