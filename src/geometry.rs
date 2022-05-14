use crate::float::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct V2f {
    pub x: f32,
    pub y: f32,
}

#[inline(always)]
pub fn v2f(x: f32, y: f32) -> V2f {
    V2f { x, y }
}

impl core::ops::Neg for V2f {
    type Output = V2f;

    #[inline(always)]
    fn neg(self) -> V2f {
        V2f {
            x: -self.x,
            y: -self.y,
        }
    }
}

impl core::ops::Add for V2f {
    type Output = V2f;

    #[inline(always)]
    fn add(self, other: V2f) -> V2f {
        V2f {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl core::ops::Sub for V2f {
    type Output = V2f;

    #[inline(always)]
    fn sub(self, other: V2f) -> V2f {
        V2f {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

impl core::ops::Mul<V2f> for V2f {
    type Output = V2f;

    #[inline(always)]
    fn mul(self, other: V2f) -> V2f {
        V2f {
            x: self.x * other.x,
            y: self.y * other.y,
        }
    }
}

impl core::ops::Div<V2f> for V2f {
    type Output = V2f;

    #[inline(always)]
    fn div(self, other: V2f) -> V2f {
        V2f {
            x: self.x / other.x,
            y: self.y / other.y,
        }
    }
}

impl core::ops::Mul<V2f> for f32 {
    type Output = V2f;

    #[inline(always)]
    fn mul(self, vec: V2f) -> V2f {
        V2f {
            x: self * vec.x,
            y: self * vec.y,
        }
    }
}

impl core::ops::Div<f32> for V2f {
    type Output = V2f;

    #[inline(always)]
    fn div(self, scalar: f32) -> V2f {
        V2f {
            x: self.x / scalar,
            y: self.y / scalar,
        }
    }
}

impl V2f {
    #[inline(always)]
    pub fn dot(self, other: V2f) -> f32 {
        (self.x * other.x) + (self.y * other.y)
    }

    #[inline(always)]
    pub fn length_squared(self) -> f32 {
        self.dot(self)
    }

    #[inline(always)]
    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    #[inline(always)]
    pub fn lerp(self, other: Self, t: f32) -> V2f {
        (1.0 - t)*self + t*other
    }

    #[inline(always)]
    pub fn min(self, other: V2f) -> V2f {
        V2f {
            x: self.x.min(other.x),
            y: self.y.min(other.y),
        }
    }

    #[inline(always)]
    pub fn max(self, other: V2f) -> V2f {
        V2f {
            x: self.x.max(other.x),
            y: self.y.max(other.y),
        }
    }

    #[inline(always)]
    pub fn clamp(self, low: V2f, high: V2f) -> V2f {
        self.max(low).min(high)
    }
}


#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub min: V2f,
    pub max: V2f,
}

#[inline(always)]
pub fn rect(min: V2f, max: V2f) -> Rect {
    Rect { min, max }
}

impl Rect {
    #[inline(always)]
    pub fn include(&mut self, p: V2f) {
        self.min = self.min.min(p);
        self.max = self.max.max(p);
    }

    #[inline(always)]
    pub fn contains(&mut self, p: V2f) -> bool {
           p.x >= self.min.x && p.x < self.max.x
        && p.y >= self.min.y && p.y < self.max.y
    }

    #[inline(always)]
    pub fn grow(self, delta: V2f) -> Rect {
        rect(self.min - delta, self.max + delta)
    }

    #[inline(always)]
    pub fn clamp_to(self, other: Rect) -> Rect {
        rect(
            self.min.clamp(other.min, other.max),
            self.max.clamp(other.min, other.max),
        )
    }

    #[inline(always)]
    pub fn round_inclusive_fast(self) -> Rect {
        rect(
            v2f(floor_fast(self.min.x), floor_fast(self.min.y)),
            v2f(ceil_fast(self.max.x),  ceil_fast(self.max.y)),
        )
    }

    #[inline(always)]
    pub fn width(self) -> f32 {
        self.max.x - self.min.x
    }

    #[inline(always)]
    pub fn height(self) -> f32 {
        self.max.y - self.min.y
    }
}


#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Segment {
    pub p0: V2f,
    pub p1: V2f,
}

#[inline(always)]
pub fn segment(p0: V2f, p1: V2f) -> Segment {
    Segment { p0, p1 }
}

impl core::ops::Add<V2f> for Segment {
    type Output = Segment;

    #[inline(always)]
    fn add(self, v: V2f) -> Segment {
        segment(
            self.p0 + v,
            self.p1 + v,
        )
    }
}



#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quadratic {
    pub p0: V2f,
    pub p1: V2f,
    pub p2: V2f,
}

#[inline(always)]
pub fn quadratic(p0: V2f, p1: V2f, p2: V2f) -> Quadratic {
    Quadratic { p0, p1, p2 }
}

impl Quadratic {
    #[inline(always)]
    pub fn split(&self, t: f32) -> (Quadratic, Quadratic) {
        let l10 = self.p0.lerp(self.p1, t);
        let l11 = self.p1.lerp(self.p2, t);
        let l20 = l10.lerp(l11, t);
        (quadratic(self.p0, l10, l20), quadratic(l20, l11, self.p2))
    }

    // u32 parameter is max_recursion.
    pub fn flatten<F: FnMut(V2f, V2f, u32)>
        (&self, f: &mut F, tolerance_squared: f32, max_recursion: u32)
    {
        /* max error occurs for t = 0.5:
            err = length(p1/2 - (p0 + p2)/4)
                = length(p1*2/4 - (p0 + p2)/4)
                = 1/2 * length(2*p1 - (p0 + p2)) */
        let err_sq = 0.25 * (2.0*self.p1 - (self.p0 + self.p2)).length_squared();

        if max_recursion == 0 || err_sq < tolerance_squared {
            f(self.p0, self.p2, max_recursion)
        }
        else {
            let (q1, q2) = self.split(0.5);
            q1.flatten(f, tolerance_squared, max_recursion - 1);
            q2.flatten(f, tolerance_squared, max_recursion - 1);
        }
    }
}

impl core::ops::Add<V2f> for Quadratic {
    type Output = Quadratic;

    #[inline(always)]
    fn add(self, v: V2f) -> Quadratic {
        quadratic(
            self.p0 + v,
            self.p1 + v,
            self.p2 + v,
        )
    }
}



#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cubic {
    pub p0: V2f,
    pub p1: V2f,
    pub p2: V2f,
    pub p3: V2f,
}

#[inline(always)]
pub fn cubic(p0: V2f, p1: V2f, p2: V2f, p3: V2f) -> Cubic {
    Cubic { p0, p1, p2, p3 }
}

impl Cubic {
    pub fn split(&self, t: f32) -> (Cubic, Cubic) {
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

            control points of first segment:
                l0 = p0
                l1 = (1-s)*p0 + s*p1
                l2 = (1-s)^2*p0 + 2*(1-s)s*p1 + s^2*p2
                l3 = (1-s)^3*p0 + 3*(1-s)^2s*p1 + 3*(1-s)s^2*p2 + s^3*p3

            max error is (wolfram alpha):
                max_err = s^3 * sqrt(3)/36 * (3(p1 - p2) + (p3 - p0)).length()
                        = s^3 * max-err-of-mid-point-approx-of-original-cubic

            now solve `max_err <= tolerance` to get the split point.
        #

        # credit:
            http://www.caffeineowl.com/graphics/2d/vectorial/cubic2quad01.html
    */

    // mid point approximation.
    pub fn approx_quad(&self) -> Quadratic {
        quadratic(
            self.p0,
            0.25*3.0*(self.p1 + self.p2) - 0.25*(self.p0 + self.p3),
            self.p3)
    }


    // u32 parameter is remaining recursion budget.
    pub fn reduce<F: FnMut(Quadratic, u32)>
        (&self, f: &mut F, tolerance_squared: f32, max_recursion: u32)
    {
        let Cubic {p0, p1, p2, p3} = *self;

        let scale: f32 = 0.0481125224324688137090957317; // sqrt(3)/36
        let err_sq = scale * (3.0*(p1 - p2) + (p3 - p0)).length_squared();

        if max_recursion == 0 || err_sq < tolerance_squared {
            f(self.approx_quad(), max_recursion);
        }
        else {
            // solve t^3 * sqrt(err_sq) = sqrt(tolerance_squared)
            //       t^5 = tolerance_squared / err_sq
            let split = (tolerance_squared / err_sq).pow(1.0/5.0);

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
                m.reduce(f, tolerance_squared, max_recursion - 1);
                f(r.approx_quad(), max_recursion);
            }
            else {
                // split in the middle for better symmetry.

                let (l, r) = self.split(0.5);
                l.reduce(f, tolerance_squared, max_recursion - 1);
                r.reduce(f, tolerance_squared, max_recursion - 1);
            }
        }
    }


    // u32 parameter is remaining recursion budget.
    pub fn flatten<F: FnMut(V2f, V2f, u32)>
        (&self, f: &mut F, tolerance_squared: f32, max_recursion: u32)
    {
        // halve tolerance, because we approximate twice.
        let tolerance_squared = tolerance_squared / 4.0;

        // make sure, we have enough splitting left to flatten the quads.
        let max_recursion = max_recursion / 2;

        let mut on_quad = |quad: Quadratic, recursion_left| {
            quad.flatten(f, tolerance_squared, max_recursion + recursion_left);
        };

        self.reduce(&mut on_quad, tolerance_squared, max_recursion);
    }
}

impl core::ops::Add<V2f> for Cubic {
    type Output = Cubic;

    #[inline(always)]
    fn add(self, v: V2f) -> Cubic {
        cubic(
            self.p0 + v,
            self.p1 + v,
            self.p2 + v,
            self.p3 + v,
        )
    }
}
