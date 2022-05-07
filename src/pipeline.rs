use crate::wide::*;
use crate::image::*;


pub struct Pipeline<'i> {
    target: ImgMut<'i>,
    pub color: F32x4,
}


impl<'i> Pipeline<'i> {
    pub fn new(target: ImgMut<'i>) -> Pipeline<'i> {
        assert!(target.format() == ImageFormat::argb_u32);

        Pipeline {
            target,
            color: F32x4::from([1.0, 1.0, 1.0, 1.0]),
        }
    }

    pub fn fill_mask(&mut self, offset: U32x2, mask: Img) {
        assert!(mask.format() == ImageFormat::a_f32);

        let bounds = self.target.bounds();

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
                            coverage[x - x0 + dx] = mask.read_xy::<f32>(x, mask_y);
                        }
                        coverage
                    }
                    else {
                        mask.read_offset_xy::<F32x8>(4*mask_x0 as usize, mask_y)
                    };

                let t = self.target.read_xy::<U32x8>(u as usize, y as usize);

                let (tr, tg, tb, ta) = argb_unpack(t);

                let (tr, tg, tb, ta) = self.run(tr, tg, tb, ta, coverage);

                let t = unsafe { argb_pack_clamped(tr, tg, tb, ta) };

                self.target.write_xy::<U32x8>(u as usize, y as usize, t);
            }
        }
    }

    fn run(&self, tr: F32x8, tg: F32x8, tb: F32x8, ta: F32x8, coverage: F32x8)
        -> (F32x8, F32x8, F32x8, F32x8)
    {
        let sr = F32x8::splat(self.color[0]);
        let sg = F32x8::splat(self.color[1]);
        let sb = F32x8::splat(self.color[2]);
        let sa = F32x8::splat(self.color[3]) * coverage;

        if 0==1 {
            return (sr*sa, sg*sa, sb*sa, sa);
        }

        let one = F32x8::splat(1.0);
        (
            sa*sr + (one - sa)*ta*tr,
            sa*sg + (one - sa)*ta*tg,
            sa*sb + (one - sa)*ta*tb,
            sa    + (one - sa)*ta,
        )
    }
}

