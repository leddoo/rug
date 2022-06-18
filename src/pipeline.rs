use crate::simd::*;
use crate::image::*;


pub fn fill_mask(target: &mut Target, offset: U32x2, mask: &Mask, color: F32x4) {
    const N: usize = Target::simd_width();
    const N32: u32 = N as u32;

    let bounds = target.bounds();

    if offset[0] >= bounds[0] || offset[1] >= bounds[1] {
        return;
    }

    let begin = offset;
    let end   = (offset + mask.bounds()).min(bounds);

    let mask_width = mask.width() as i32;

    for y in begin[1] .. end[1] {
        let u0 = begin[0] / N32;
        let u1 = (end[0] + N32-1) / N32;

        for u in u0..u1 {
            let x = u * N32;

            let mask_x0 = x as i32 - begin[0] as i32;
            let mask_x1 = mask_x0 + N as i32;

            let mask_y = (y - begin[1]) as usize;

            let coverage =
                if mask_x0 < 0 || mask_x1 > mask_width {
                    let dx = (-mask_x0).max(0) as usize;
                    let x0 = mask_x0.max(0) as usize;
                    let x1 = mask_x1.min(mask_width) as usize;

                    let mut coverage = <F32x<N>>::ZERO;
                    for x in x0..x1 {
                        coverage[x - x0 + dx] = mask[(x, mask_y)];
                    }
                    coverage
                }
                else {
                    mask.read(mask_x0 as usize, mask_y)
                };

            let p = (u as usize, y as usize);

            if coverage.lanes_lt(<F32x<N>>::splat(0.5/255.0)).all() {
                continue;
            }
            if color[3] == 1.0 && coverage.lanes_gt(<F32x<N>>::splat(254.5/255.0)).all() {
                target[p] = [
                    <F32x<N>>::splat(color[0]),
                    <F32x<N>>::splat(color[1]),
                    <F32x<N>>::splat(color[2]),
                    <F32x<N>>::splat(1.0),
                ];
                continue;
            }

            target[p] = run(target[p], coverage, color);
        }
    }
}

use core::simd::{LaneCount, SupportedLaneCount};

fn run<const N: usize>(t: [F32x<N>; 4], coverage: F32x<N>, color: F32x4) -> [F32x<N>; 4]
    where LaneCount<N>: SupportedLaneCount
{
    let [tr, tg, tb, ta] = t;

    let sr = <F32x<N>>::splat(color[0]);
    let sg = <F32x<N>>::splat(color[1]);
    let sb = <F32x<N>>::splat(color[2]);
    let sa = <F32x<N>>::splat(color[3]) * coverage;

    let one = <F32x<N>>::splat(1.0);
    [
        sa*sr + (one - sa)*ta*tr,
        sa*sg + (one - sa)*ta*tg,
        sa*sb + (one - sa)*ta*tb,
        sa    + (one - sa)*ta,
    ]
}

