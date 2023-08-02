use sti::simd::*;


#[inline(always)]
pub fn argb_unpack(v: u32) -> F32x4 {
    let a = (v >> 24) & 0xff;
    let r = (v >> 16) & 0xff;
    let g = (v >>  8) & 0xff;
    let b = (v >>  0) & 0xff;

    U32x4::new(r, g, b, a).as_i32().to_f32() / 255.0
}

#[inline(always)]
pub unsafe fn argb_pack_clamped_255(v: F32x4) -> u32 {
    let [r, g, b, a] = *v;

    // @temp: float utils.
    unsafe {
        let a = a.to_int_unchecked::<i32>() << 24;
        let r = r.to_int_unchecked::<i32>() << 16;
        let g = g.to_int_unchecked::<i32>() <<  8;
        let b = b.to_int_unchecked::<i32>() <<  0;
        (a | r | g | b) as u32
    }
}

#[inline(always)]
pub fn argb_pack(v: F32x4) -> u32 {
    let offset = F32x4::splat(0.5);
    let scale = F32x4::splat(255.0);
    let min = F32x4::splat(0.0);
    let max = F32x4::splat(255.0);
    unsafe { argb_pack_clamped_255(
        (scale*v + offset).clamp(min, max)
    )}
}


#[inline(always)]
pub fn argb_u8x4_unpack(v: U32x4) -> [F32x4; 4] {
    let mask = U32x4::splat(0xff);
    let a = (v >> 24) & mask;
    let r = (v >> 16) & mask;
    let g = (v >>  8) & mask;
    let b = (v >>  0) & mask;

    let scale = F32x4::splat(255.0);
    [r.as_i32().to_f32() / scale,
     g.as_i32().to_f32() / scale,
     b.as_i32().to_f32() / scale,
     a.as_i32().to_f32() / scale]
}


#[inline(always)]
pub fn argb_pack_u8s(r: u8, g: u8, b: u8, a: u8) -> u32 {
    let (r, g, b, a) = (r as u32, g as u32, b as u32, a as u32);
    a << 24 | r << 16 | g << 8 | b
}

#[inline(always)]
pub unsafe fn argb_u8x4_pack_clamped_255(v: [F32x4; 4]) -> U32x4 {
    let [r, g, b, a] = v;

    let a = a.to_i32_unck() << 24;
    let r = r.to_i32_unck() << 16;
    let g = g.to_i32_unck() <<  8;
    let b = b.to_i32_unck() <<  0;
    (a | r | g | b).as_u32()
}

#[inline(always)]
pub fn argb_u8x4_pack(v: [F32x4; 4]) -> U32x4 {
    let offset = F32x4::splat(0.5);
    let scale = F32x4::splat(255.0);
    let min = F32x4::splat(0.0);
    let max = F32x4::splat(255.0);
    let [r, g, b, a] = v;
    unsafe { argb_u8x4_pack_clamped_255([
        (scale*r + offset).clamp(min, max),
        (scale*g + offset).clamp(min, max),
        (scale*b + offset).clamp(min, max),
        (scale*a + offset).clamp(min, max),
    ]) }
}


#[inline(always)]
pub unsafe fn abgr_u8x4_pack_clamped_255(v: [F32x4; 4]) -> U32x4 {
    let [r, g, b, a] = v;

    let a = a.to_i32_unck() << 24;
    let b = b.to_i32_unck() << 16;
    let g = g.to_i32_unck() <<  8;
    let r = r.to_i32_unck() <<  0;
    (a | b | g | r).as_u32()
}

#[inline(always)]
pub fn abgr_u8x4_pack(v: [F32x4; 4]) -> U32x4 {
    let offset = F32x4::splat(0.5);
    let scale = F32x4::splat(255.0);
    let min = F32x4::splat(0.0);
    let max = F32x4::splat(255.0);
    let [r, g, b, a] = v;
    unsafe { abgr_u8x4_pack_clamped_255([
        (scale*r + offset).clamp(min, max),
        (scale*g + offset).clamp(min, max),
        (scale*b + offset).clamp(min, max),
        (scale*a + offset).clamp(min, max),
    ]) }
}


