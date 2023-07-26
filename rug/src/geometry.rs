use sti::simd::*;


trait F32x2Ext {
    fn normalized(self) -> Self;
    fn rotated_acw(self) -> Self;
    fn rotated_cw(self) -> F32x2;

    fn left_normal_unck(self) -> F32x2;
    fn left_normal(self, tolerance_sq: f32) -> Option<F32x2>;
}

impl F32x2Ext for F32x2 {
    #[inline(always)]
    fn normalized(self) -> Self {
        self / self.length()
    }

    #[inline(always)]
    fn rotated_acw(self) -> Self {
        Self::new(-self.y(), self.x())
    }

    #[inline(always)]
    fn rotated_cw(self) -> F32x2 {
        F32x2::new(self.y(), -self.x())
    }

    #[inline(always)]
    fn left_normal_unck(self) -> F32x2 {
        self.normalized().rotated_acw()
    }

    #[inline(always)]
    fn left_normal(self, tolerance_sq: f32) -> Option<F32x2> {
        if self.length_sq() > tolerance_sq {
            return Some(self.left_normal_unck());
        }
        None
    }

}


#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub min: F32x2,
    pub max: F32x2,
}

#[inline(always)]
pub const fn rect(min: F32x2, max: F32x2) -> Rect {
    Rect { min, max }
}

impl Rect {
    pub const ZERO: Rect = rect(F32x2::ZERO, F32x2::ZERO);

    /// invalid rect, useful for constructing aabbs with Rect::include.
    pub const MAX_MIN: Rect = rect(F32x2::MAX, F32x2::MIN);

    #[inline(always)]
    pub fn valid(self) -> bool {
        self.min.le(self.max).all()
    }

    #[inline(always)]
    pub fn from_points(p0: F32x2, p1: F32x2) -> Rect {
        rect(p0.min(p1), p0.max(p1))
    }

    #[inline(always)]
    pub fn include(&mut self, p: F32x2) {
        self.min = self.min.min(p);
        self.max = self.max.max(p);
    }

    #[inline(always)]
    pub fn contains(&self, p: F32x2) -> bool {
        p.ge(self.min).all() && p.lt(self.max).all()
    }

    #[inline(always)]
    pub fn contains_inclusive(&self, p: F32x2) -> bool {
        p.ge(self.min).all() && p.le(self.max).all()
    }

    #[inline(always)]
    pub fn grow(self, delta: F32x2) -> Rect {
        rect(self.min - delta, self.max + delta)
    }

    #[inline(always)]
    pub fn clamp_to(self, other: Rect) -> Rect {
        rect(
            self.min.clamp(other.min, other.max),
            self.max.clamp(other.min, other.max))
    }

    #[inline(always)]
    pub fn round_inclusive(self) -> Rect {
        rect(
            self.min.floor(),
            self.max.ceil())
    }

    #[inline(always)]
    pub fn size(self) -> F32x2 {
        self.max - self.min
    }

    #[inline(always)]
    pub fn width(self) -> f32 {
        self.size().x()
    }

    #[inline(always)]
    pub fn height(self) -> f32 {
        self.size().y()
    }
}


#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Line {
    pub p0: F32x2,
    pub p1: F32x2,
}

#[inline(always)]
pub fn line(p0: F32x2, p1: F32x2) -> Line {
    Line { p0, p1 }
}

impl Line {
    #[inline(always)]
    pub fn normal(self, tolerance_sq: f32) -> Option<F32x2> {
        (self.p1 - self.p0).left_normal(tolerance_sq)
    }

    #[inline(always)]
    pub fn offset(self, normal: F32x2, distance: f32) -> Line {
        self + distance*normal
    }

    #[inline(always)]
    pub fn aabb(self) -> Rect {
        rect(
            self.p0.min(self.p1),
            self.p0.max(self.p1))
    }

    #[inline(always)]
    pub fn rev(self) -> Line {
        line(self.p1, self.p0)
    }

    pub fn ggb(self) {
        println!("Segment(({}, {}), ({}, {})),",
            self.p0.x(), self.p0.y(),
            self.p1.x(), self.p1.y());
    }
}

impl core::ops::Add<F32x2> for Line {
    type Output = Line;

    #[inline(always)]
    fn add(self, v: F32x2) -> Line {
        line(
            self.p0 + v,
            self.p1 + v)
    }
}



#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quad {
    pub p0: F32x2,
    pub p1: F32x2,
    pub p2: F32x2,
}

#[inline(always)]
pub fn quad(p0: F32x2, p1: F32x2, p2: F32x2) -> Quad {
    Quad { p0, p1, p2 }
}

impl Quad {
    #[inline(always)]
    pub fn eval(self, t: f32) -> F32x2 {
        let l10 = self.p0.lerp(self.p1, t);
        let l11 = self.p1.lerp(self.p2, t);
        l10.lerp(l11, t)
    }

    #[inline(always)]
    pub fn split(self, t: f32) -> (Quad, Quad) {
        let l10 = self.p0.lerp(self.p1, t);
        let l11 = self.p1.lerp(self.p2, t);
        let l20 = l10.lerp(l11, t);
        (quad(self.p0, l10, l20), quad(l20, l11, self.p2))
    }


    // u32 parameter is max_recursion.
    pub fn flatten<F: FnMut(F32x2, F32x2, u32)>
        (self, tolerance_sq: f32, max_recursion: u32, f: &mut F)
    {
        /* max error occurs for t = 0.5:
            err = length(p1/2 - (p0 + p2)/4)
                = length(p1*2/4 - (p0 + p2)/4)
                = 1/2 * length(2*p1 - (p0 + p2)) */
        let err_sq = 0.25 * (2.0*self.p1 - (self.p0 + self.p2)).length_sq();

        if max_recursion == 0 || err_sq < tolerance_sq {
            f(self.p0, self.p2, max_recursion)
        }
        else {
            let (q1, q2) = self.split(0.5);
            q1.flatten(tolerance_sq, max_recursion - 1, f);
            q2.flatten(tolerance_sq, max_recursion - 1, f);
        }
    }


    #[inline(always)]
    pub fn normals(self, tolerance_sq: f32) -> (Option<F32x2>, Option<F32x2>) {
        ((self.p1 - self.p0).left_normal(tolerance_sq),
         (self.p2 - self.p1).left_normal(tolerance_sq))
    }

    pub fn offset<F: FnMut(Quad, u32)>(
        self, f: &mut F, normal_start: F32x2, normal_end: F32x2, distance: f32,
        tolerance_sq: f32, max_recursion: u32
    ) {
        debug_assert!((self.p2 - self.p0).length_sq() > tolerance_sq);

        let n0 = normal_start;
        let n2 = normal_end;

        // TODO: understand & explain.
        let n1 = n0 + n2;
        let n1 = 2.0*(n1 / n1.dot(n1));

        let d = F32x2::splat(distance);
        let approx =
            quad(
                self.p0 + d*n0,
                self.p1 + d*n1,
                self.p2 + d*n2);

        let mid = self.eval(0.5);
        let n_mid = (self.p2 - self.p0).left_normal_unck();

        let expected = mid + d*n_mid;
        let actual   = approx.eval(0.5);

        // TODO: keep this? (ensures (p2 - p0) is large enough in the recursive calls)
        let l_smol = (mid - self.p0).length_sq() <= tolerance_sq;
        let r_smol = (self.p2 - mid).length_sq() <= tolerance_sq;

        let max_dev = actual - expected;
        if max_recursion == 0 || max_dev.length_sq() <= tolerance_sq || l_smol || r_smol {
            f(approx, max_recursion);
        }
        else {
            // TODO: split at point closest to p1?
            let (l, r) = self.split(0.5);
            l.offset(f, n0, n_mid, distance, tolerance_sq, max_recursion - 1);
            r.offset(f, n_mid, n2, distance, tolerance_sq, max_recursion - 1);
        }
    }


    #[inline(always)]
    pub fn aabb(self) -> Rect {
        rect(
            self.p0.min(self.p1).min(self.p2),
            self.p0.max(self.p1).max(self.p2))
    }

    #[inline(always)]
    pub fn rev(self) -> Quad {
        quad(self.p2, self.p1, self.p0)
    }

    pub fn ggb(self) {
        println!("Curve((1 - t)² ({}, {}) + 2(1 - t) t ({}, {}) + t² ({}, {}), t, 0, 1),",
            self.p0.x(), self.p0.y(),
            self.p1.x(), self.p1.y(),
            self.p2.x(), self.p2.y());
    }
}

impl core::ops::Add<F32x2> for Quad {
    type Output = Quad;

    #[inline(always)]
    fn add(self, v: F32x2) -> Quad {
        quad(
            self.p0 + v,
            self.p1 + v,
            self.p2 + v)
    }
}



#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cubic {
    pub p0: F32x2,
    pub p1: F32x2,
    pub p2: F32x2,
    pub p3: F32x2,
}

#[inline(always)]
pub fn cubic(p0: F32x2, p1: F32x2, p2: F32x2, p3: F32x2) -> Cubic {
    Cubic { p0, p1, p2, p3 }
}

impl Cubic {
    pub fn split(self, t: f32) -> (Cubic, Cubic) {
        let l10 = self.p0.lerp(self.p1, t);
        let l11 = self.p1.lerp(self.p2, t);
        let l12 = self.p2.lerp(self.p3, t);
        let l20 = l10.lerp(l11, t);
        let l21 = l11.lerp(l12, t);
        let l30 = l20.lerp(l21, t);
        (cubic(self.p0, l10, l20, l30), cubic(l30, l21, l12, self.p3))
    }


    /* approximation of cubics with quadratics:

        # polynomial forms:
            q3(t) = p0 + 3t(p1 - p0) + 3t^2(p0 + p2 - 2*p1)
                    + t^3(3*(p1 - p2) - (p0 - p3))

            q2(t) = p0 + 2t(p1 - p0) + t^2(p0 + p2 - 2*p1)
        #

        # approximate by dropping 3rd degree term & equating:
            1) p0 = l0
            2) 3(p1 - p0) = 2(l1 - l0)
            3) 3(p0 + p2 - 2*p1) = (l0 + l2 - 2*l1)

            2) l1 = ( 3(p1 - p0) + 2*p0 )/2
                  = ( 3*p1 - 3*p0 + 2*p0 )/2
                  = ( 3*p1 - p0 )/2

            3) l2 = 3(p0 + p2 - 2*p1) - p0 + 2*( ( 3*p1 - p0 )/2 )
                  = 3*p0 + 3*p2 - 6*p1 - p0 + 3*p1 - p0
                  = p0 - 3*p1 + 3*p2
                  = p0 + 3(p2 - p1)

            The quadratic (l0, l1, l2) is the left approximation.
            It is close to the original cubic for t values close to 0.

            cubic (p0, p1, p2, p3) = cubic (p3, p2, p1, p0)
            Thus, we get the right approximation (r0, r1, r2) with:
                r0 = p3
                r1 = ( 3*p2 - p3 )/2
                r2 = p3 + 3(p1 - p2)
        #

        # determining error:
            We've only dropped the cubic term, so the greater the cubic term,
            the greater the error.
            Thus we get the largest error for t=1:
                left error  = 3*(p1 - p2) - (p0 - p3)
                right error = 3*(p2 - p1) - (p3 - p0)
            Because we only care about the magnitude, these are equal.
            Neither approximation is particularly desirable, because errors at
            the end points are more noticable in paths consisting of multiple
            curves.
        #

        # error for quad (p0, c, p3):
            err(t)
            = (1-t)^2*p0 + 2(1-t)t*c + t^2*p3
              - (1-t)^3*p0 - 3(1-t)^2t*p1 - 3(1-t)t^2*p2 - t^3*p3
            = (1-t)t * (2*c - 3(1-t)*p1 - 3t*p2)
              + (1-t)^2*p0 + t^2*p3
              - (1-t)^3*p0 - t^3*p3
            = (1-t)t * (2*c - 3(1-t)*p1 - 3t*p2)
              + (1-t)^2*p0 - (1-t)^3*p0
              + t^2*p3 - t^3*p3
            = (1-t)t * (2*c - 3(1-t)*p1 - 3t*p2)
              + (1 - (1-t)) * (1-t)^2*p0
              + (1-t) * t^2*p3
            = (1-t)t * (2*c - 3(1-t)*p1 - 3t*p2)
              + (1-t)t * (1-t)*p0
              + (1-t)t * t*p3
            = (1-t)t * (2*c - 3(1-t)*p1 - 3t*p2 + (1-t)*p0 + t*p3)
            = (1-t)t * (2*c - 3*p1 + 3t*p1 - 3t*p2 + p0 - t*p0 + t*p3)
            = (1-t)t * (2*c - 3*p1 + p0 + t*(3*(p1 - p2) + (p3 - p0)))
        #

        # the mid-point approximation:
            This approximation seems optimal. Though I don't know of a proof.
            Some observations:
                - it "groups" with the inner `t*...` term of the error function
                  (see below).
                - the plot of the error function appears to always be a line.
                  for other control points, the error function usually contains
                  a loop.

            let c = (l1 + r1) / 2
                  = (3(p1 + p2) - (p0 + p3))/4:

            err(t)
            = (1-t)t * ((3(p1 + p2) - (p0 + p3))/2 - 3*p1 + p0 + t*(3*(p1 - p2) + (p3 - p0)))
            = (1-t)t * (3/2p1 + 3/2p2 - p0/2 - p3/2 - 3*p1 + p0 + t*(3*(p1 - p2) + (p3 - p0)))
            = (1-t)t * ((3*(p2 - p1) + (p0 - p3))/2 + t*(3*(p1 - p2) + (p3 - p0)))
            = (1-t)t * ((-3*(p1 - p2) - (p3 - p0))/2 + t*(3*(p1 - p2) + (p3 - p0)))
            = (1-t)t * (-1/2*(3*(p1 - p2) + (p3 - p0)) + t*(3*(p1 - p2) + (p3 - p0)))
            = (1-t)t * (t - 1/2) * (3*(p1 - p2) + (p3 - p0))

            find max t:
                f(t) = (1-t)t * (t - 1/2)
                = (t - t^2) * (t - 1/2)
                = t^2 - 1/2*t - t^3 + 1/2*t^2

                f'(t) = -1/2 + 3*t - 3*t^2
                p = -1
                q = 1/6
                t = 1/2 +/- sqrt(1/4 - 1/6)
                  = 1/2 +/- sqrt(6/24 - 4/24)
                  = 1/2 +/- sqrt(2/24)
                  = 1/2 +/- sqrt(1/(4*3))
                  = (1 +/- 1/sqrt(3))/2

                f((1 +/- 1/sqrt(3))/2) = +/- sqrt(3)/36

            max absolute error:
                sqrt(3)/36 * (3*(p1 - p2) + (p3 - p0)).length()
        #

        # adaptive splitting
            goal: find maximum t for which error of mid-point approximation of
            first segment is within tolerance.

            control points of first segment (split at t):
                l0 = p0
                l1 = (1-t)*p0 + t*p1
                l2 = (1-t)^2*p0 + 2*(1-t)t*p1 + t^2*p2
                l3 = (1-t)^3*p0 + 3*(1-t)^2t*p1 + 3*(1-t)t^2*p2 + t^3*p3

            max error is (wolfram alpha):
                # TODO: explain what's put into wolfram alpha.
                max_err = t^3 * sqrt(3)/36 * (3(p1 - p2) + (p3 - p0)).length()
                        = t^3 * max-err-of-mid-point-approx-of-original-cubic

            now solve `max_err <= tolerance` to get the split point.
        #

        # credit:
            http://www.caffeineowl.com/graphics/2d/vectorial/cubic2quad01.html
    */

    // mid point approximation.
    pub fn approx_quad(self) -> Quad {
        quad(
            self.p0,
            (0.25*3.0)*(self.p1 + self.p2) - 0.25*(self.p0 + self.p3),
            self.p3)
    }


    // u32 parameter is remaining recursion budget.
    pub fn reduce<F: FnMut(Quad, u32)>
        (self, f: &mut F, tolerance_sq: f32, max_recursion: u32)
    {
        let Cubic {p0, p1, p2, p3} = self;

        // TODO: this should be squared, i think.
        // maybe that's why pow 1/12 is better.
        let scale: f32 = 0.0481125224324688137090957317; // sqrt(3)/36
        let err_sq = scale * (3.0*(p1 - p2) + (p3 - p0)).length_sq();

        if max_recursion == 0 || err_sq < tolerance_sq {
            f(self.approx_quad(), max_recursion);
        }
        else {
            // solve t^3 * sqrt(err_sq) = sqrt(tolerance_sq)
            //       t^6 = tolerance_sq/ err_sq
            // experimentally, 1/12 is the best. weird. TODO: investigate.
            let split = (tolerance_sq / err_sq).powf(1.0/12.0);

            if split < 0.5 {
                // we can use symmetry to split twice!

                // 0    t       1-t   1
                // |----|--------|----|
                //      |- 1-2t -|
                //      |---- 1-t ----|
                let split_2 = (1.0 - 2.0*split) / (1.0 - split);

                let (l, r) = self.split(split);
                let (m, r) = r.split(split_2);

                f(l.approx_quad(), max_recursion);
                m.reduce(f, tolerance_sq, max_recursion - 1);
                f(r.approx_quad(), max_recursion);
            }
            else {
                // split in the middle for better symmetry.

                let (l, r) = self.split(0.5);
                l.reduce(f, tolerance_sq, max_recursion - 1);
                r.reduce(f, tolerance_sq, max_recursion - 1);
            }
        }
    }

    // u32 parameter is remaining recursion budget.
    pub fn flatten<F: FnMut(F32x2, F32x2, u32)>
        (self, tolerance_sq: f32, max_recursion: u32, f: &mut F)
    {
        // halve tolerance, because we approximate twice.
        let tolerance_sq = tolerance_sq / 4.0;

        // make sure, we have enough splitting left to flatten the quads.
        let max_recursion = max_recursion / 2;

        let mut on_quad = |quad: Quad, recursion_left| {
            quad.flatten(tolerance_sq, max_recursion + recursion_left, f);
        };

        self.reduce(&mut on_quad, tolerance_sq, max_recursion);
    }


    #[inline(always)]
    pub fn aabb(self) -> Rect {
        rect(
            (self.p0.min(self.p1)).min(self.p2.min(self.p3)),
            (self.p0.max(self.p1)).max(self.p2.max(self.p3)))
    }

    #[inline(always)]
    pub fn rev(self) -> Cubic {
        cubic(self.p3, self.p2, self.p1, self.p0)
    }

    pub fn ggb(self) {
        println!("Curve((1 - t)³ ({}, {}) + 3(1 - t)² t ({}, {}) + 3 (1 - t) t² ({}, {}) + t³ ({}, {}), t, 0, 1),",
            self.p0.x(), self.p0.y(),
            self.p1.x(), self.p1.y(),
            self.p2.x(), self.p2.y(),
            self.p3.x(), self.p3.y());
    }
}

impl core::ops::Add<F32x2> for Cubic {
    type Output = Cubic;

    #[inline(always)]
    fn add(self, v: F32x2) -> Cubic {
        cubic(
            self.p0 + v,
            self.p1 + v,
            self.p2 + v,
            self.p3 + v)
    }
}


#[derive(Clone, Copy, Debug)]
pub struct Transform {
    pub columns: [F32x2; 3],
}

impl Transform {
    pub const ID: Transform = Transform::scale(1.0);

    #[inline(always)]
    pub const fn scale(s: f32) -> Transform {
        Transform { columns: [
            F32x2::from_array([  s, 0.0]),
            F32x2::from_array([0.0,   s]),
            F32x2::from_array([0.0, 0.0]),
        ]}
    }

    #[inline(always)]
    pub const fn translate(v: F32x2) -> Transform {
        let mut result = Transform::ID;
        result.columns[2] = v;
        result
    }

    #[inline(always)]
    pub fn aabb_transform(self, aabb: Rect) -> Rect {
        let p0 = self * F32x2::new(aabb.min.x(), aabb.min.y());
        let p1 = self * F32x2::new(aabb.min.x(), aabb.max.y());
        let p2 = self * F32x2::new(aabb.max.x(), aabb.min.y());
        let p3 = self * F32x2::new(aabb.max.x(), aabb.max.y());
        rect(
            (p0.min(p1)).min(p2.min(p3)),
            (p0.max(p1)).max(p2.max(p3)))
    }
}

impl core::ops::Index<usize> for Transform {
    type Output = F32x2;

    #[inline(always)]
    fn index(&self, column: usize) -> &F32x2 {
        &self.columns[column]
    }
}

impl core::ops::IndexMut<usize> for Transform {
    #[inline(always)]
    fn index_mut(&mut self, column: usize) -> &mut F32x2 {
        &mut self.columns[column]
    }
}

impl core::ops::Mul<Transform> for Transform {
    type Output = Transform;

    #[inline(always)]
    fn mul(self, other: Transform) -> Transform {
        let r0 = F32x2::new(self[0][0], self[1][0]);
        let r1 = F32x2::new(self[0][1], self[1][1]);
        Transform { columns: [
            F32x2::new(r0.dot(other[0]), r1.dot(other[0])),
            F32x2::new(r0.dot(other[1]), r1.dot(other[1])),
            F32x2::new(r0.dot(other[2]), r1.dot(other[2])) + self[2],
        ]}
    }
}

impl core::ops::Mul<F32x2> for Transform {
    type Output = F32x2;

    #[inline(always)]
    fn mul(self, v: F32x2) -> F32x2 {
        F32x2::splat(v[0])*self[0] + F32x2::splat(v[1])*self[1] + self[2]
    }
}

impl core::ops::Mul<Line> for Transform {
    type Output = Line;

    #[inline(always)]
    fn mul(self, s: Line) -> Line {
        line(self*s.p0, self*s.p1)
    }
}

impl core::ops::Mul<Quad> for Transform {
    type Output = Quad;

    #[inline(always)]
    fn mul(self, q: Quad) -> Quad {
        quad(self*q.p0, self*q.p1, self*q.p2)
    }
}

impl core::ops::Mul<Cubic> for Transform {
    type Output = Cubic;

    #[inline(always)]
    fn mul(self, c: Cubic) -> Cubic {
        cubic(self*c.p0, self*c.p1, self*c.p2, self*c.p3)
    }
}

