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
        //println!("Segment(({}, {}), ({}, {})),", p0.x(), p0.y(), p1.x(), p1.y());
        //println!("Vector(({}, {}), ({}, {})),", p0.x(), p0.y(), p1.x(), p1.y());

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

    pub fn stroke_path(&mut self,
        path: &Path, left: f32, right: f32, position: F32x2,
    ) {
        //println!("stroke {} {}", left, right);
        let left  = left.max(0.0);
        let right = right.max(0.0);

        let left  = if left  > self.flatten_tolerance { left }  else { 0.0 };
        let right = if right > self.flatten_tolerance { right } else { 0.0 };

        if left == 0.0 && right == 0.0 {
            //println!("reject");
            return;
        }

        let mut iter = path.iter();
        while iter.has_next() {
            let begin_verb = iter.verb;

            let mut stroker = Stroker {
                closed:            false,
                has_prev:          false,
                prev_end:          F32x2::zero(),
                has_first:         false,
                first_left:        F32x2::zero(),
                first_right:       F32x2::zero(),
                tolerance_squared: self.flatten_tolerance.squared(),
                max_recursion:     self.flatten_recursion,
                distance:          left,
                rasterizer:        self,
            };

            // left offset.
            loop {
                match iter.next().unwrap() {
                    IterEvent::Begin (_, closed) => {
                        assert!(iter.verb == begin_verb + 1);
                        stroker.closed = closed;
                    },

                    IterEvent::Segment (s) => {
                        stroker.segment(s + -position);
                    },

                    IterEvent::Quadratic (q) => {
                        stroker.quadratic(q + -position);
                    },

                    IterEvent::Cubic (c) => {
                        stroker.cubic(c + -position);
                    },

                    IterEvent::End (_) => {
                        break;
                    },
                }
            }

            if !stroker.has_prev || iter.verb - begin_verb <= 2 {
                continue;
            }

            // right offset.
            stroker.has_prev = false;
            stroker.distance = right;

            // manual path::RevIter, because that doesn't exist.
            let mut verb  = iter.verb - 1;
            let mut point = iter.point;

            // process end
            assert!(path.verbs[verb] == Verb::End);
            let mut rev_p0 = path.points[point - 1];
            point -= 1;
            verb -= 1;

            while verb > begin_verb {
                match path.verbs[verb] {
                    Verb::Begin | Verb::BeginClosed => unreachable!(),

                    Verb::Segment => {
                        let p0 = rev_p0;
                        let p1 = path.points[point - 1];
                        point -= 1;
                        rev_p0 = p1;
                        stroker.segment(segment(p0, p1) + -position);
                    },

                    Verb::Quadratic => {
                        let p0 = rev_p0;
                        let p1 = path.points[point - 1];
                        let p2 = path.points[point - 2];
                        point -= 2;
                        rev_p0 = p2;
                        stroker.quadratic(quadratic(p0, p1, p2) + -position);
                    },

                    Verb::Cubic => {
                        let p0 = rev_p0;
                        let p1 = path.points[point - 1];
                        let p2 = path.points[point - 2];
                        let p3 = path.points[point - 3];
                        point -= 3;
                        rev_p0 = p3;
                        stroker.cubic(cubic(p0, p1, p2, p3) + -position)
                    },

                    Verb::End => unreachable!(),
                }
                verb -= 1;
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


struct Stroker<'r, 'a> {
    rasterizer:        &'r mut Rasterizer<'a>,
    closed:            Bool,
    has_prev:          Bool,
    prev_end:          F32x2,
    has_first:         Bool,
    first_left:        F32x2,
    first_right:       F32x2,
    tolerance_squared: F32,
    max_recursion:     u32,
    distance:          F32,
}

impl<'r, 'a> Stroker<'r, 'a> {
    #[inline(always)]
    fn cap(&mut self, p0: F32x2, normal: F32x2) {
        // only for first curve.
        if !self.has_prev {
            self._cap(p0, normal);
        }
    }

    fn _cap(&mut self, p0: F32x2, normal: F32x2) {
        // closed paths are joined; open paths get caps.
        if self.closed {
            if !self.has_first {
                // remember end points during left offset.
                self.has_first   = true;
                self.first_left  = p0 + self.distance*normal;
                self.first_right = p0 - self.distance*normal;
            }
            else {
                // draw join during right offset.
                self.has_first = false;
                let left  = p0 + self.distance*normal;
                let right = p0 - self.distance*normal;
                self.rasterizer.add_segment_p(right, self.first_left);
                self.rasterizer.add_segment_p(self.first_right, left);
            }
        }
        else {
            // butt cap.
            let left  = p0 + self.distance*normal;
            let right = p0 - self.distance*normal;
            self.rasterizer.add_segment_p(right, left);
        }
    }

    #[inline(always)]
    fn join(&mut self, p0: F32x2, normal: F32x2) {
        if self.has_prev {
            // bevel join.
            self.rasterizer.add_segment_p(self.prev_end, p0 + self.distance*normal);
        }
    }

    #[inline(always)]
    fn set_end(&mut self, end: F32x2) {
        self.has_prev = true;
        self.prev_end = end;
    }

    #[inline(always)]
    fn segment(&mut self, segment: Segment) {
        if let Some(normal) = segment.normal(self.tolerance_squared) {
            self._segment(segment, normal);
        }
    }

    fn _segment(&mut self, segment: Segment, normal: F32x2) {
        self.cap(segment.p0, normal);

        if self.distance != 0.0 {
            self.join(segment.p0, normal);
            self.rasterizer.add_segment(segment.offset(normal, self.distance));
            self.set_end(segment.p1 + self.distance*normal);
        }
        else {
            self.rasterizer.add_segment(segment);
            self.set_end(segment.p1);
        }
    }

    #[inline(always)]
    fn quadratic(&mut self, quadratic: Quadratic) {
        self._quadratic(quadratic, self.tolerance_squared, self.max_recursion)
    }

    fn _quadratic(&mut self, quadratic: Quadratic, tolerance_squared: F32, max_recursion: u32) {
        let Quadratic { p0, p1, p2 } = quadratic;

        if (p2 - p0).length_squared() <= self.tolerance_squared {
            self.segment(segment(p0, p1));
            self.segment(segment(p1, p2));
            return;
        }

        match quadratic.normals(self.tolerance_squared) {
            (Some(n0), Some(n1)) => {
                self.cap(quadratic.p0, n0);

                if self.distance != 0.0 {
                    self.join(quadratic.p0, n0);

                    let tol = tolerance_squared / 4.0;
                    let rec = max_recursion / 2;
                    let mut f = |q, rec_left| {
                        self.rasterizer.add_quadratic_tol_rec(q, tol, rec + rec_left);
                    };
                    quadratic.offset(&mut f, n0, n1, self.distance, tol, rec);

                    self.set_end(quadratic.p2 + self.distance*n1);
                }
                else {
                    self.rasterizer.add_quadratic_tol_rec(quadratic, tolerance_squared, max_recursion);
                    self.set_end(quadratic.p2);
                }
            },

            (Some(n0), None) => {
                self._segment(segment(p0, p2), n0);
            },

            (None, Some(n1)) => {
                self._segment(segment(p0, p2), n1);
            },

            _ => (), // should be unreachable, because p0 = p1 = p2, but p0 ≠ p2
        }
    }

    fn cubic(&mut self, cubic: Cubic) {
        let tol = self.tolerance_squared / 4.0;
        let rec = self.max_recursion / 2;
        let mut f = |q, rec_left| {
            self._quadratic(q, tol, rec + rec_left);
        };
        cubic.reduce(&mut f, tol, rec);
    }
}
