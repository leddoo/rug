use crate::wide::*;
use crate::image::*;


pub fn fill_mask(target: &mut Target, offset: U32x2, mask: &Mask, color: F32x4) {
    let bounds = target.bounds();

    if offset[0] >= bounds[0] || offset[1] >= bounds[1] {
        return;
    }

    let begin = offset;
    let end   = (offset + mask.bounds()).min(bounds);

    let mask_width = mask.width() as i32;

    for y in begin[1] .. end[1] {
        let u0 = begin[0] / 8;
        let u1 = (end[0] + 7) / 8;

        for u in u0..u1 {
            let x = u * 8;

            let mask_x0 = x as i32 - begin[0] as i32;
            let mask_x1 = mask_x0 + 8;

            let mask_y = (y - begin[1]) as usize;

            let coverage =
                if mask_x0 < 0 || mask_x1 > mask_width {
                    let dx = (-mask_x0).max(0) as usize;
                    let x0 = mask_x0.max(0) as usize;
                    let x1 = mask_x1.min(mask_width) as usize;

                    let mut coverage = F32x8::default();
                    for x in x0..x1 {
                        coverage[x - x0 + dx] = mask[(x, mask_y)];
                    }
                    coverage
                }
                else {
                    mask.read8(mask_x0 as usize, mask_y)
                };

            let p = (u as usize, y as usize);

            let (tr, tg, tb, ta) = target[p];

            let (tr, tg, tb, ta) = run(tr, tg, tb, ta, coverage, color);

            target[p] = (tr, tg, tb, ta);
        }
    }
}

fn run(tr: F32x8, tg: F32x8, tb: F32x8, ta: F32x8, coverage: F32x8, color: F32x4)
    -> (F32x8, F32x8, F32x8, F32x8)
{
    let sr = F32x8::splat(color[0]);
    let sg = F32x8::splat(color[1]);
    let sb = F32x8::splat(color[2]);
    let sa = F32x8::splat(color[3]) * coverage;

    /*
    if color[3] == 1.0 && coverage.lanes_gt(F32x8::splat(1.0 - 1.0/255.0)).all() {
        return (sr, sg, sb, F32x8::splat(1.0));
    }
    else if coverage.lanes_lt(F32x8::splat(1.0/255.0)).all() {
        return (tr, tg, tb, ta);
    }
    */

    let one = F32x8::splat(1.0);
    (
        sa*sr + (one - sa)*ta*tr,
        sa*sg + (one - sa)*ta*tg,
        sa*sb + (one - sa)*ta*tb,
        sa    + (one - sa)*ta,
    )
}

