use sti::simd::*;
use sti::float::*;

use crate::geometry::*;
use crate::color::*;
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
pub fn render(cmds: &[Cmd], params: &RenderParams, target: &mut ImgMut<u32>) {
    let clear = [
        F32x4::splat(argb_unpack(params.clear)[0]),
        F32x4::splat(argb_unpack(params.clear)[1]),
        F32x4::splat(argb_unpack(params.clear)[2]),
        F32x4::splat(argb_unpack(params.clear)[3]),
    ];
    let mut render_image = <Image<[F32x4; 4], _>>::with_clear(*target.size(), clear);

    let mut raster_image = Image::new([0, 0]);
    let clip = Rect { min: F32x2::ZERO, max: target.size().as_i32().to_f32() };

    let tfx = &params.tfx;

    for cmd in cmds {
        match *cmd {
            Cmd::FillPathSolid { path, color } => {
                // todo: aabb bounds check.
                let aabb = tfx.aabb_transform(path.aabb());

                let (raster_size, raster_origin, blit_offset) =
                    raster_rect_for(aabb, clip, 4);

                let mut tfx = *tfx;
                tfx.columns[2] -= raster_origin;

                let mut r = Rasterizer::new(&mut raster_image, *raster_size);
                r.fill_path(path, &tfx);
                let mask = r.accumulate();

                let color = argb_unpack(color);

                fill_mask_solid(&mask.img(), blit_offset, color, &mut render_image.img_mut());
            }

            _ => unimplemented!()
        }
    }

    // writeback.
    target.copy_expand(&render_image.img(), I32x2::ZERO,
        |c| *abgr_u8x4_pack(c));
}


/// - `clip` must be a valid integer rect with `clip.min >= zero`.
/// - `align` is the horizontal alignment in pixels (for simd blitting).
/// - returns `(raster_size, raster_origin, blit_offset)`.
///     - `raster_size`   is the size of the rasterizer's rect.
///     - `raster_origin` is the global position of the rasterizer's origin.
///     - `blit_offset`   is the integer offset from `clip` to the rasterizer's origin.
pub fn raster_rect_for(rect: Rect, clip: Rect, align: u32) -> (U32x2, F32x2, U32x2) {
    // compute rasterizer's integer rect in global coordinates.
    let raster_rect = rect.clamp_to(clip).round_inclusive();

    // pad rasterizer rect to meet alignment requirement.
    let align = align as f32;
    let x0 = (raster_rect.min.x() / align).ffloor() * align;
    let x1 = (raster_rect.max.x() / align).fceil()  * align;

    let raster_size   = F32x2::new(x1 - x0, raster_rect.height()).to_i32_unck().as_u32();
    let raster_origin = F32x2::new(x0,      raster_rect.min.y());
    let blit_offset   = (raster_origin - clip.min).to_i32_unck().as_u32();
    (raster_size, raster_origin, blit_offset)
}


pub fn fill_mask_solid(mask: &Img<f32>, offset: U32x2, color: F32x4, target: &mut ImgMut<[F32x4; 4]>) {
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



