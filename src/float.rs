#[inline(always)]
pub fn safe_div(num: f32, denom: f32, default: f32) -> f32 {
    if denom != 0.0 {
        num / denom
    }
    else {
        default
    }
}

#[inline(always)]
pub fn squared(v: f32) -> f32 {
    v*v
}
