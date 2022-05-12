pub trait Float {
    fn copysign(self, other: Self) -> Self;

    fn abs(self) -> Self;
    fn floor(self) -> Self;
    fn ceil(self) -> Self;

    fn safe_div(self, denom: Self, default: Self) -> Self;

    fn squared(self) -> Self;
    fn sqrt(self) -> Self;

    fn pow(self, n: Self) -> Self;
}

impl Float for f32 {
    fn copysign(self, other: f32) -> f32 {
        libm::copysignf(self, other)
    }

    fn abs(self) -> f32 { libm::fabsf(self) }
    fn floor(self) -> f32 { libm::floorf(self) }
    fn ceil(self) -> f32 { libm::ceilf(self) }

    fn safe_div(self, denom: f32, default: f32) -> f32 {
        if denom != 0.0 {
            self / denom
        }
        else {
            default
        }
    }

    fn squared(self: f32) -> f32 { self*self }
    fn sqrt(self) -> f32 { libm::sqrtf(self) }

    fn pow(self, n: f32) -> f32 { libm::powf(self, n) }
}


#[inline(always)]
pub fn floor_fast(a: f32) -> f32 {
    let i = unsafe { a.to_int_unchecked::<i32>() as f32 };
    i - (a < i) as i32 as f32
}

#[inline(always)]
pub fn ceil_fast(a: f32) -> f32 {
    let i = unsafe { a.to_int_unchecked::<i32>() as f32 };
    i + (a > i) as i32 as f32
}
