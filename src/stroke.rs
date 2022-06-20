extern crate alloc;
use alloc::alloc::*;

use basic::{*, simd::*};

use crate::geometry::*;
use crate::path::*;
use crate::rasterizer::ZERO_TOLERANCE_SQ;


pub fn stroke_path(path: &Path, width: F32) -> SoaPath<'static> {
    stroke_path_in(path, width, &Global)
}

pub fn stroke_path_in<'a>(path: &Path, width: F32, allocator: &'a dyn Allocator) -> SoaPath<'a> {
    let mut stroker = Stroker {
        left:   width/2.0,
        right: -width/2.0,
        tolerance_sq: 0.05 * 0.05,
        max_recursion: 16,

        ..Stroker::default(allocator)
    };

    for e in path.iter() {
        match e {
            IterEvent::Begin (_, closed) => { stroker.begin_path(closed); }
            IterEvent::Segment (s)       => { stroker.segment(s); },
            IterEvent::Quadratic (q)     => { stroker.quadratic(q); },
            IterEvent::Cubic (c)         => { stroker.cubic(c); },
            IterEvent::End (_)           => { stroker.end_path() },
        }
    }

    stroker.done()
}


struct Stroker<'a> {
    lines: Vec<Segment,   &'a dyn Allocator>,
    quads: Vec<Quadratic, &'a dyn Allocator>,
    aabb: Rect,

    pub left:   F32,
    pub right:  F32,
    pub closed: Bool,

    pub tolerance_sq:  F32,
    pub max_recursion: U32,

    has_prev:    Bool,
    prev_left:   F32x2,
    prev_right:  F32x2,
    first_left:  F32x2,
    first_right: F32x2,
}

impl<'a> Stroker<'a> {
    fn default(allocator: &'a dyn Allocator) -> Self {
        Self {
            lines: Vec::new_in(allocator),
            quads: Vec::new_in(allocator),
            aabb: rect(F32x2::splat(f32::MAX), F32x2::splat(f32::MIN)),

            left:   0.0,
            right:  0.0,
            closed: false,

            tolerance_sq:  0.0,
            max_recursion: 0,

            has_prev:    false,
            prev_left:   F32x2::ZERO,
            prev_right:  F32x2::ZERO,
            first_left:  F32x2::ZERO,
            first_right: F32x2::ZERO,
        }
    }
}

impl<'a> Stroker<'a> {
    fn done(self) -> SoaPath<'a> {
        let alloc = *self.lines.allocator();
        SoaPath {
            lines:  self.lines.into_boxed_slice(),
            quads:  self.quads.into_boxed_slice(),
            cubics: Vec::new_in(alloc).into_boxed_slice(),
            aabb:   self.aabb,
        }
    }

    #[inline(always)]
    fn begin_path(&mut self, closed: Bool) {
        self.has_prev = false;
        self.closed = closed;
    }

    fn end_path(&mut self) {
        // final cap or join
        if self.has_prev {
            if self.closed {
                // bevel join.
                self.push_line(segment(self.prev_left,  self.first_left));
                self.push_line(segment(self.prev_right, self.first_right).rev());
            }
            else {
                // butt cap.
                self.push_line(segment(self.prev_left, self.prev_right));
            }
        }
    }

    #[inline(always)]
    fn push_line(&mut self, line: Segment) {
        self.aabb.include(line.p0);
        self.aabb.include(line.p1);
        self.lines.push(line);
    }

    #[inline(always)]
    fn push_quad(&mut self, quad: Quadratic) {
        self.aabb.include(quad.p0);
        self.aabb.include(quad.p1);
        self.aabb.include(quad.p2);
        self.quads.push(quad);
    }


    #[inline(always)]
    fn cap(&mut self, p0: F32x2, normal: F32x2) {
        if !self.has_prev {
            self._cap(p0, normal);
        }
    }

    #[inline(never)]
    fn _cap(&mut self, p0: F32x2, normal: F32x2) {
        // closed paths are joined; open paths get caps.
        let left  = p0 + self.left.mul(normal);
        let right = p0 + self.right.mul(normal);

        if self.closed {
            // remember end points.
            self.first_left  = left;
            self.first_right = right;
        }
        else {
            // butt cap.
            self.push_line(segment(right, left));
        }
    }

    #[inline(always)]
    fn join(&mut self, p0: F32x2, normal: F32x2) {
        if self.has_prev {
            // bevel join.
            self.push_line(segment(self.prev_left,  p0 + self.left.mul(normal)));
            self.push_line(segment(self.prev_right, p0 + self.right.mul(normal)).rev());
        }
    }

    #[inline(always)]
    fn set_end(&mut self, end: F32x2, normal: F32x2) {
        self.has_prev = true;
        self.prev_left  = end + self.left.mul(normal);
        self.prev_right = end + self.right.mul(normal);
    }


    #[inline(always)]
    fn segment(&mut self, segment: Segment) {
        if let Some(normal) = segment.normal(ZERO_TOLERANCE_SQ) {
            self._segment(segment, normal);
        }
    }

    #[inline(never)]
    fn _segment(&mut self, segment: Segment, normal: F32x2) {
        self.cap(segment.p0, normal);

        self.join(segment.p0, normal);
        self.push_line(segment.offset(normal, self.left));
        self.push_line(segment.offset(normal, self.right).rev());
        self.set_end(segment.p1, normal);
    }
    

    #[inline(always)]
    fn quadratic(&mut self, quadratic: Quadratic) {
        self._quadratic(quadratic, self.tolerance_sq, self.max_recursion)
    }

    #[inline(never)]
    fn _quadratic(&mut self, quadratic: Quadratic, tolerance_sq: F32, max_recursion: u32) {
        let Quadratic { p0, p1, p2 } = quadratic;

        if (p2 - p0).length_squared() <= ZERO_TOLERANCE_SQ {
            self.segment(segment(p0, p1));
            self.segment(segment(p1, p2));
            return;
        }

        match quadratic.normals(ZERO_TOLERANCE_SQ) {
            (Some(n0), Some(n1)) => {
                self.cap(quadratic.p0, n0);

                self.join(quadratic.p0, n0);

                let tol = tolerance_sq;
                let rec = max_recursion;
                let (l, r) = (self.left, self.right);

                let mut f = |q, _| { self.push_quad(q); };
                quadratic.offset(&mut f, n0, n1, l, tol, rec);

                let mut f = |q: Quadratic, _| { self.push_quad(q.rev()); };
                quadratic.offset(&mut f, n0, n1, r, tol, rec);

                self.set_end(quadratic.p2, n1);
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

    #[inline(never)]
    fn cubic(&mut self, cubic: Cubic) {
        let tol = self.tolerance_sq / 4.0;
        let rec = self.max_recursion / 2;
        let mut f = |q, rec_left| { self._quadratic(q, tol, rec + rec_left); };
        cubic.reduce(&mut f, tol, rec);
    }
}
