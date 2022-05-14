pub use core::simd::u8x8  as U8x8;
pub use core::simd::u8x32 as U8x32;

pub use core::simd::u32x2 as U32x2;
pub use core::simd::u32x4 as U32x4;
pub use core::simd::u32x8 as U32x8;


pub type Bool = bool;
pub type N32 = u32;
pub type I32 = i32;
pub type F32 = f32;


macro_rules! wide_create {
    ($Self: ty, $Ctor: expr, $Scalar: ty, $width: expr, $from_array: expr, $splat: expr, $zero: expr) => {
        impl $Self {
            #[inline(always)]
            pub fn from_array(vs: [$Scalar; $width]) -> $Self {
                $Ctor($from_array(vs))
            }

            #[inline(always)]
            pub fn splat(v: $Scalar) -> $Self {
                $Ctor($splat(v))
            }

            #[inline(always)]
            pub fn zero() -> $Self {
                $Ctor($splat($zero))
            }
        }

        impl Default for $Self {
            fn default() -> $Self {
                $Ctor($splat($zero))
            }
        }
    };
}

macro_rules! wide_compare {
    ($Self: ty, $Ctor: expr, $Bool: ty, $BoolCtor: expr) => {
        impl $Self {
            #[inline(always)]
            pub fn eq(self, other: $Self) -> $Bool {
                $BoolCtor(self.0.lanes_eq(other.0))
            }

            #[inline(always)]
            pub fn le(self, other: $Self) -> $Bool {
                $BoolCtor(self.0.lanes_le(other.0))
            }

            #[inline(always)]
            pub fn lt(self, other: $Self) -> $Bool {
                $BoolCtor(self.0.lanes_lt(other.0))
            }

            #[inline(always)]
            pub fn ge(self, other: $Self) -> $Bool {
                $BoolCtor(self.0.lanes_ge(other.0))
            }

            #[inline(always)]
            pub fn gt(self, other: $Self) -> $Bool {
                $BoolCtor(self.0.lanes_gt(other.0))
            }
        }
    };
}

macro_rules! wide_ops {
    ($Self: ty, $Ctor: expr, $Scalar: ty, $splat: expr) => {
        impl core::ops::Add<$Self> for $Self {
            type Output = $Self;

            #[inline(always)]
            fn add(self, other: $Self) -> $Self {
                $Ctor(self.0 + other.0)
            }
        }

        impl core::ops::Sub<$Self> for $Self {
            type Output = $Self;

            #[inline(always)]
            fn sub(self, other: $Self) -> $Self {
                $Ctor(self.0 - other.0)
            }
        }

        impl core::ops::Mul<$Self> for $Self {
            type Output = $Self;

            #[inline(always)]
            fn mul(self, other: $Self) -> $Self {
                $Ctor(self.0 * other.0)
            }
        }

        impl core::ops::Div<$Self> for $Self {
            type Output = $Self;

            #[inline(always)]
            fn div(self, other: $Self) -> $Self {
                $Ctor(self.0 / other.0)
            }
        }


        impl core::ops::Mul<$Self> for $Scalar {
            type Output = $Self;

            #[inline(always)]
            fn mul(self, other: $Self) -> $Self {
                $splat(self) * other
            }
        }

        impl core::ops::Mul<$Scalar> for $Self {
            type Output = $Self;

            #[inline(always)]
            fn mul(self, other: $Scalar) -> $Self {
                self * $splat(other)
            }
        }

        impl core::ops::Div<$Scalar> for $Self {
            type Output = $Self;

            #[inline(always)]
            fn div(self, other: $Scalar) -> $Self {
                self / $splat(other)
            }
        }

        impl core::ops::Index<usize> for $Self {
            type Output = $Scalar;

            #[inline(always)]
            fn index(&self, index: usize) -> &$Scalar {
                &self.0[index]
            }
        }

        impl core::ops::IndexMut<usize> for $Self {
            #[inline(always)]
            fn index_mut(&mut self, index: usize) -> &mut $Scalar {
                &mut self.0[index]
            }
        }
    };
}

macro_rules! wide_min_max {
    ($Self: ty, $Ctor: expr, $Scalar: ty) => {
        impl $Self {
            #[inline(always)]
            pub fn min(self, other: $Self) -> $Self {
                $Ctor(self.0.min(other.0))
            }

            #[inline(always)]
            pub fn max(self, other: $Self) -> $Self {
                $Ctor(self.0.max(other.0))
            }

            #[inline(always)]
            pub fn clamp(self, low: $Self, high: $Self) -> $Self {
                $Ctor(self.0.clamp(low.0, high.0))
            }


            #[inline(always)]
            pub fn hadd(self) -> $Scalar {
                self.0.reduce_sum()
            }

            #[inline(always)]
            pub fn hmul(self) -> $Scalar {
                self.0.reduce_product()
            }

            #[inline(always)]
            pub fn hmin(self) -> $Scalar {
                self.0.reduce_min()
            }

            #[inline(always)]
            pub fn hmax(self) -> $Scalar {
                self.0.reduce_max()
            }
        }
    };
}

macro_rules! wide_signed {
    ($Self: ty, $Ctor: expr) => {
        impl core::ops::Neg for $Self {
            type Output = $Self;

            #[inline(always)]
            fn neg(self) -> $Self {
                $Ctor(-self.0)
            }
        }

        impl $Self {
            #[inline(always)]
            pub fn abs(self) -> $Self {
                $Ctor(self.0.abs())
            }
        }
    };
}

macro_rules! wide_float {
    ($Self: ty, $Ctor: expr, $Scalar: ty) => {
        impl $Self {
            #[inline(always)]
            pub fn floor_fast(self) -> $Self {
                let i = unsafe { self.0.to_int_unchecked::<i32>().cast() };
                $Ctor(i + self.0.lanes_lt(i).to_int().cast())
            }

            #[inline(always)]
            pub fn ceil_fast(self) -> $Self {
                let i = unsafe { self.0.to_int_unchecked::<i32>().cast() };
                $Ctor(i - self.0.lanes_gt(i).to_int().cast())
            }


            #[inline(always)]
            pub fn dot(self, other: $Self) -> $Scalar {
                (self * other).hadd()
            }

            #[inline(always)]
            pub fn length_squared(self) -> $Scalar {
                self.dot(self)
            }

            #[inline(always)]
            pub fn length(self) -> $Scalar {
                self.length_squared().sqrt()
            }

            #[inline(always)]
            pub fn lerp(self, other: Self, t: $Scalar) -> $Self {
                (1.0 - t)*self + t*other
            }
        }
    };
}

macro_rules! wide_field {
    ($Self: ty, $Scalar: ty, $name: ident, $name_mut: ident, $index: expr) => {
        impl $Self {
            #[inline(always)]
            pub fn $name(self) -> $Scalar { self[$index] }

            #[inline(always)]
            pub fn $name_mut(&mut self) -> &mut $Scalar { &mut self[$index] }
        }
    };
}


#[derive(Clone, Copy, Debug, PartialEq)]
pub struct B32x2 (pub core::simd::mask32x2);

impl B32x2 {
    #[inline(always)]
    pub fn any(self) -> Bool {
        self.0.any()
    }

    #[inline(always)]
    pub fn all(self) -> Bool {
        self.0.all()
    }
}


#[derive(Clone, Copy, Debug, PartialEq)]
pub struct B32x4 (pub core::simd::mask32x4);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct B32x8 (pub core::simd::mask32x8);



#[derive(Clone, Copy, Debug, PartialEq)]
pub struct F32x2 (pub core::simd::f32x2);

impl F32x2 {
    #[inline(always)]
    pub fn new(v0: F32, v1: F32) -> F32x2 {
        Self::from_array([v0, v1])
    }
}

wide_create!(F32x2, F32x2, F32, 2, core::simd::f32x2::from_array, core::simd::f32x2::splat, 0.0);
wide_compare!(F32x2, F32x2, B32x2, B32x2);
wide_ops!(F32x2, F32x2, F32, F32x2::splat);
wide_min_max!(F32x2, F32x2, F32);
wide_signed!(F32x2, F32x2);
wide_float!(F32x2, F32x2, F32);
wide_field!(F32x2, F32, x, x_mut, 0);
wide_field!(F32x2, F32, y, y_mut, 1);


#[derive(Clone, Copy, Debug, PartialEq)]
pub struct F32x4 (pub core::simd::f32x4);

impl F32x4 {
    #[inline(always)]
    pub fn new(v0: F32, v1: F32, v2: F32, v3: F32) -> F32x4 {
        Self::from_array([v0, v1, v2, v3])
    }
}

wide_create!(F32x4, F32x4, F32, 4, core::simd::f32x4::from_array, core::simd::f32x4::splat, 0.0);
wide_compare!(F32x4, F32x4, B32x4, B32x4);
wide_ops!(F32x4, F32x4, F32, F32x4::splat);
wide_min_max!(F32x4, F32x4, F32);
wide_signed!(F32x4, F32x4);
wide_float!(F32x4, F32x4, F32);


#[derive(Clone, Copy, Debug, PartialEq)]
pub struct F32x8 (pub core::simd::f32x8);

wide_create!(F32x8, F32x8, F32, 8, core::simd::f32x8::from_array, core::simd::f32x8::splat, 0.0);
wide_compare!(F32x8, F32x8, B32x8, B32x8);
wide_ops!(F32x8, F32x8, F32, F32x8::splat);
wide_min_max!(F32x8, F32x8, F32);
wide_signed!(F32x8, F32x8);
wide_float!(F32x8, F32x8, F32);

