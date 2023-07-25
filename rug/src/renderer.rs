use sti::simd::*;

use crate::geometry::*;
use crate::image::*;
use crate::cmd::*;
use crate::rasterizer::Rasterizer;


#[derive(Clone, Copy)]
pub struct RenderParams {
    pub clear: u32,

    pub tfx: Transform,

    // target format.
    //  eventually maybe take DynImgMut, which is an enum,
    //  cause there's a statically known set of supported 
    //  image formats -- but that also means,
    //  the `target` param already specifies the format).

    // composite mode (whether base color is clear or old target values).
}

// right, so this is the function, we'd put in a trait.
// which means a renderer is a struct, which would enable
// allocation caching, for example.
pub fn render(cmds: &[Cmd], params: RenderParams, target: &mut ImgMut<u32>) {
    let mut render_image = <Image<[F32x4; 4], _>>::new(*target.size());

    let mut raster_image = Image::new([0, 0]);

    for cmd in cmds {
        match cmd {
            Cmd::FillPathSolid { path, color } => {
                // aabb bounds check.
                // ~ rasterizer::rect_for

                let mut r = Rasterizer::new(&mut raster_image, [0, 0]);
                r.fill_path(*path, &params.tfx);
                let mask = r.accumulate();

                //fill_mask_solid(mask.img(), render_image.img_mut());
            }

            _ => unimplemented!()
        }
    }
}


pub fn fill_mask_solid(mask: Img<f32>, offset: U32x2, color: F32x4, target: &mut ImgMut<[F32x4; 4]>) {
    let n = 4;

    let size = U32x2::new(n*target.width(), target.height());

    let begin = offset;
    let end   = (offset + mask.size()).min(size);
    if begin.eq(end).any() {
        return;
    }

    let u0 = begin.x() / n;
    let u1 = end.x()   / n;
    assert!(u0 * n == begin.x());
    assert!(u1 * n == end.x());

    for y in begin.y() .. end.y() {
        for u in u0..u1 {
            let x = u * n;
            let mask_x = (x - begin.x()) as usize;
            let mask_y = (y - begin.y()) as usize;

            let coverage = F32x4::from_array(mask.read_n(mask_x, mask_y));

            let p = (u as usize, y as usize);

            if coverage.lt(F32x4::splat(0.5/255.0)).all() {
                continue;
            }
            if color[3] == 1.0 && coverage.gt(F32x4::splat(254.5/255.0)).all() {
                target[p] = [
                    F32x4::splat(color[0]),
                    F32x4::splat(color[1]),
                    F32x4::splat(color[2]),
                    F32x4::splat(1.0),
                ];
                continue;
            }

            let [tr, tg, tb, ta] = target[p];

            let sr = F32x4::splat(color[0]);
            let sg = F32x4::splat(color[1]);
            let sb = F32x4::splat(color[2]);
            let sa = F32x4::splat(color[3]) * coverage;

            let one = F32x4::splat(1.0);
            target[p] = [
                sa*sr + (one - sa)*ta*tr,
                sa*sg + (one - sa)*ta*tg,
                sa*sb + (one - sa)*ta*tb,
                sa    + (one - sa)*ta,
            ];
        }
    }
}



