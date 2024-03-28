use sti::simd::*;
use sti::float::*;

use crate::geometry::*;
use crate::color::*;
use crate::image::*;
use crate::path::Path;
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

/* implementation notes:

    pre-multiplied alpha:
        the renderer's internal buffers use pre-multiplied alpha.
        input colors in the command buffer are *not* pre-multiplied.
        that would result in information loss, as the input colors
        (currently) only have 8 bit color depth.
        furthermore, color interpolation, like in gradients, must be
        done on the non-pre-multiplied colors.

    srgb vs linear:
        tbd.
*/

// right, so this is the function, we'd put in a trait.
// which means a renderer is a struct, which would enable
// allocation caching, for example.
pub fn render(cmd_buf: &CmdBuf, params: &RenderParams, target: &mut ImgMut<u32>) {
    let clear = argb_unpack_premultiply(params.clear);
    let clear = [
        F32x4::splat(clear[0]),
        F32x4::splat(clear[1]),
        F32x4::splat(clear[2]),
        F32x4::splat(clear[3]),
    ];

    let mut render_image = {
        let w = (target.width() + 3) / 4;
        let h = target.height();
        <Image<[F32x4; 4], _>>::with_clear([w, h], clear)
    };

    let mut raster_image = Image::new([0, 0]);
    let clip = Rect { min: F32x2::ZERO(), max: target.size().as_i32().to_f32() };

    let mut gradient_stop_buffer = Vec::new();

    let tfx = &params.tfx;

    for i in 0..cmd_buf.num_cmds() {
        match *cmd_buf.cmd(i) {
            Cmd::FillPathSolid { path, color } => {
                let aabb = tfx.aabb_transform(path.aabb());

                let (raster_size, raster_origin, blit_offset) =
                    raster_rect_for(aabb, clip, 4);

                if raster_size.eq(U32x2::ZERO()).any() { continue }

                let mut tfx = *tfx;
                tfx.columns[2] -= raster_origin;

                let mut r = Rasterizer::new(&mut raster_image, *raster_size);
                r.fill_path(path, &tfx);
                let mask = r.accumulate();

                let color = argb_unpack_premultiply(color);

                fill_mask_solid(&mask.img(), blit_offset, color, &mut render_image.img_mut());
            }

            Cmd::StrokePathSolid { path, color, width } => {
                let stroke = crate::stroke::stroke(path, width);
                let path = stroke.path();

                let aabb = tfx.aabb_transform(path.aabb());

                let (raster_size, raster_origin, blit_offset) =
                    raster_rect_for(aabb, clip, 4);

                if raster_size.eq(U32x2::ZERO()).any() { continue }

                let mut tfx = *tfx;
                tfx.columns[2] -= raster_origin;

                let mut r = Rasterizer::new(&mut raster_image, *raster_size);
                r.fill_path(path, &tfx);
                let mask = r.accumulate();

                let color = argb_unpack_premultiply(color);

                fill_mask_solid(&mask.img(), blit_offset, color, &mut render_image.img_mut());
            }

            Cmd::FillPathLinearGradient { path, gradient, opacity } => {
                let aabb = tfx.aabb_transform(path.aabb());

                let (raster_size, raster_origin, blit_offset) =
                    raster_rect_for(aabb, clip, 4);

                if raster_size.eq(U32x2::ZERO()).any() { continue }

                let mut tfx = *tfx;
                tfx.columns[2] -= raster_origin;

                let mut r = Rasterizer::new(&mut raster_image, *raster_size);
                r.fill_path(path, &tfx);
                let mask = r.accumulate();

                let gradient = cmd_buf.linear_gradient(gradient);
                let stops = gradient.stops;

                let p0 = (tfx * gradient.tfx) * gradient.p0;
                let p1 = (tfx * gradient.tfx) * gradient.p1;

                if stops.len() == 2 {
                    let s0 = stops[0];
                    let s1 = stops[1];
                    let c0 = argb_unpack(s0.color);
                    let c1 = argb_unpack(s1.color);
                    fill_mask_linear_gradient_2(
                        p0.lerp(p1, s0.offset), p0.lerp(p1, s1.offset),
                        c0, c1, opacity,
                        &mask.img(), blit_offset, &mut render_image.img_mut());
                }
                else if stops.len() > 0 {
                    gradient_stop_buffer.clear();
                    for stop in stops {
                        gradient_stop_buffer.push(GradientStopF32 {
                            offset: stop.offset,
                            color:  argb_unpack(stop.color),
                        });
                    }

                    fill_mask_linear_gradient_n(
                        p0, p1,
                        &gradient_stop_buffer, opacity,
                        &mask.img(), blit_offset, &mut render_image.img_mut());
                }
            }

            Cmd::FillPathRadialGradient { path, gradient, opacity } => {
                let Some(inv_tfx) = tfx.invert(0.00001) else { continue };

                let aabb = tfx.aabb_transform(path.aabb());

                let (raster_size, raster_origin, blit_offset) =
                    raster_rect_for(aabb, clip, 4);

                if raster_size.eq(U32x2::ZERO()).any() { continue }

                let mut tfx = *tfx;
                tfx.columns[2] -= raster_origin;

                let mut r = Rasterizer::new(&mut raster_image, *raster_size);
                r.fill_path(path, &tfx);
                let mask = r.accumulate();

                let gradient = cmd_buf.radial_gradient(gradient);
                let stops = gradient.stops;

                if let Some(inv_grad_tfx) = gradient.tfx.invert(0.00001) {
                    if stops.len() == 2 {
                        let s0 = stops[0];
                        let s1 = stops[1];
                        let c0 = argb_unpack(s0.color);
                        let c1 = argb_unpack(s1.color);
                        fill_mask_radial_gradient_2(
                            raster_origin, inv_tfx, inv_grad_tfx,
                            gradient,
                            GradientStopF32 { offset: s0.offset, color: c0 },
                            GradientStopF32 { offset: s1.offset, color: c1 },
                            opacity,
                            &mask.img(),
                            blit_offset,
                            &mut render_image.img_mut());
                    }
                    else if stops.len() > 0 {
                        gradient_stop_buffer.clear();
                        for stop in stops {
                            gradient_stop_buffer.push(GradientStopF32 {
                                offset: stop.offset,
                                color:  argb_unpack(stop.color),
                            });
                        }

                        fill_mask_radial_gradient_n(
                            raster_origin, inv_tfx, inv_grad_tfx,
                            gradient, &gradient_stop_buffer,
                            opacity,
                            &mask.img(), blit_offset, &mut render_image.img_mut());
                    }
                }
                else {
                    //println!("skipping radial gradient with degenerate transform");
                }
            }
        }
    }

    // writeback.
    {
        // @todo: un-premultiply for non-opaque clear.
        target.copy_expand(&render_image.img(), I32x2::ZERO(),
            |c| *abgr_u8x_pack(c));
    }
}


pub struct RenderTarget {
    size: [u32; 2],
    image: Image<[F32x4; 4]>,
    tfx: Transform,
    user_clip: Rect,
    net_clip: Rect, // `clip` clipped to `image`
    raster_cache: Image<f32>,
}

impl RenderTarget {
    pub fn new() -> Self {
        Self {
            size: [0, 0],
            image: Image::with_clear([0, 0], [F32x::ZERO(); 4]),
            tfx: Transform::ID(),
            user_clip: Rect { min: F32x2::ZERO(), max: F32x2::MAX() },
            net_clip: Rect::ZERO(),
            raster_cache: Image::new([0, 0]),
        }
    }

    pub fn size(&self) -> [u32; 2] {
        self.size
    }

    pub fn resize(&mut self, new_size: [u32; 2], clear: u32) {
        if new_size != self.size {
            let w = (new_size[0] + 3) / 4;
            let h = new_size[1];
            let clear = argb_unpack_premultiply(clear);
            let clear = [
                F32x::splat(clear[0]),
                F32x::splat(clear[1]),
                F32x::splat(clear[2]),
                F32x::splat(clear[3]),
            ];
            self.size = new_size;
            self.image.resize_and_clear([w, h], clear);
            self.net_clip = self.user_clip.clamp_to(self.image_clip());
        }
    }

    pub fn clear(&mut self, clear: u32) {
        let clear = argb_unpack_premultiply(clear);
        let clear = [
            F32x::splat(clear[0]),
            F32x::splat(clear[1]),
            F32x::splat(clear[2]),
            F32x::splat(clear[3]),
        ];
        self.image.clear(clear);
    }


    #[inline]
    pub fn transform(&self) -> Transform {
        self.tfx
    }

    /// returns the old transform.
    #[inline]
    pub fn set_transform(&mut self, new_tfx: Transform) -> Transform {
        core::mem::replace(&mut self.tfx, new_tfx)
    }

    #[inline]
    pub fn clip(&self) -> Rect {
        self.user_clip
    }

    /// returns the old clip.
    #[inline]
    pub fn set_clip(&mut self, new_clip: Rect) -> Rect {
        self.net_clip = new_clip.clamp_to(self.image_clip());
        core::mem::replace(&mut self.user_clip, new_clip)
    }

    fn image_clip(&self) -> Rect {
        Rect { min: F32x2::ZERO(), max: U32x2::from_array(self.size).as_i32().to_f32() }
    }


    /*
    pub fn draw_line(&mut self, p0: F32x2, p1: F32x2, color: u32) {
        todo!()
    }

    pub fn fill_rect(&mut self, p0: F32x2, size: F32x2, color: u32) {
        todo!()
    }

    pub fn stroke_rect(&mut self, p0: F32x2, size: F32x2, color: u32) {
        todo!()
    }
    */

    pub fn fill_path(&mut self, path: Path, color: u32) {
        fill_path_solid(path, color, self.tfx, self.net_clip, &mut self.raster_cache, &mut self.image.img_mut());
    }

    pub fn stroke_path(&mut self, path: Path, width: f32, color: u32) {
        stroke_path_solid(path, width, color, self.tfx, self.net_clip, &mut self.raster_cache, &mut self.image.img_mut());
    }


    pub fn write_to_image(&self, dst: &mut ImgMut<u32>) {
        dst.copy_expand(&self.image.img(), I32x2::ZERO(), |c| *abgr_u8x_pack(c));
    }
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


pub fn fill_path_solid<const N: usize>(
    path: Path,
    color: u32,
    tfx: Transform,
    clip: Rect,
    raster_image_cache: &mut Image<f32>,
    target: &mut ImgMut<[F32x<N>; 4]>)
where (): SimdLanes<N>
{
    let aabb = tfx.aabb_transform(path.aabb());

    let (raster_size, raster_origin, blit_offset) =
        raster_rect_for(aabb, clip, 4);

    if raster_size.eq(U32x2::ZERO()).any() { return }

    let mut tfx = tfx;
    tfx.columns[2] -= raster_origin;

    let mut r = Rasterizer::new(raster_image_cache, *raster_size);
    r.fill_path(path, &tfx);
    let mask = r.accumulate();

    let color = argb_unpack_premultiply(color);

    fill_mask_solid(&mask.img(), blit_offset, color, target);
}

pub fn stroke_path_solid<const N: usize>(
    path: Path,
    width: f32,
    color: u32,
    tfx: Transform,
    clip: Rect,
    raster_image_cache: &mut Image<f32>,
    target: &mut ImgMut<[F32x<N>; 4]>)
where (): SimdLanes<N>
{
    let stroke = crate::stroke::stroke(path, width);
    let path = stroke.path();

    let aabb = tfx.aabb_transform(path.aabb());

    let (raster_size, raster_origin, blit_offset) =
        raster_rect_for(aabb, clip, 4);

    if raster_size.eq(U32x2::ZERO()).any() { return }

    let mut tfx = tfx;
    tfx.columns[2] -= raster_origin;

    let mut r = Rasterizer::new(raster_image_cache, *raster_size);
    r.fill_path(path, &tfx);
    let mask = r.accumulate();

    let color = argb_unpack_premultiply(color);

    fill_mask_solid(&mask.img(), blit_offset, color, target);
}


/// - input pre-multiplied alpha: yes.
pub fn fill_mask_solid<const N: usize>(
    mask: &Img<f32>,
    offset: U32x2,
    color: F32x4,
    target: &mut ImgMut<[F32x<N>; 4]>)
where (): SimdLanes<N>
{
    let n = N as u32;

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

            let coverage = F32x::from_array(mask.read_n(mask_x, mask_y));

            let p = (u as usize, y as usize);

            if coverage.lt(F32x::splat(0.5/255.0)).all() {
                continue;
            }
            if color[3] == 1.0 && coverage.gt(F32x::splat(254.5/255.0)).all() {
                target[p] = [
                    F32x::splat(color[0]),
                    F32x::splat(color[1]),
                    F32x::splat(color[2]),
                    F32x::splat(1.0),
                ];
                continue;
            }

            let [tr, tg, tb, ta] = target[p];

            let sr = F32x::splat(color[0]) * coverage;
            let sg = F32x::splat(color[1]) * coverage;
            let sb = F32x::splat(color[2]) * coverage;
            let sa = F32x::splat(color[3]) * coverage;

            let one = F32x::splat(1.0);
            target[p] = [
                sr + (one - sa)*tr,
                sg + (one - sa)*tg,
                sb + (one - sa)*tb,
                sa + (one - sa)*ta,
            ];
        }
    }
}



#[derive(Clone, Copy, Debug)]
pub struct GradientStopF32 {
    pub offset:  f32,
    pub color:   F32x4,
}

/// - input pre-multiplied alpha: no.
pub fn fill_mask_linear_gradient_2<const N: usize>(
    p0: F32x2,
    p1: F32x2,
    color_0: F32x4,
    color_1: F32x4,
    opacity: f32,
    mask: &Img<f32>,
    offset: U32x2,
    target: &mut ImgMut<[F32x<N>; 4]>)
where (): SimdLanes<N>
{
    let n = N as u32;

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

    let px0 = F32x::from_array(core::array::from_fn(|i| i as f32 + 0.5));

    let mut py = F32x::splat(0.5);

    for y in begin.y() .. end.y() {
        let mut px = px0;

        for u in u0..u1 {
            let x = u * n;
            let mask_x = (x - begin.x()) as usize;
            let mask_y = (y - begin.y()) as usize;

            let coverage = F32x::from_array(mask.read_n(mask_x, mask_y));

            let p = (u as usize, y as usize);

            if coverage.lt(F32x::splat(0.5/255.0)).all() {
                px += F32x::splat(n as f32);
                continue;
            }

            // skipping alpha blending does not appear to be worth it.
            // @todo: try on lower spec hardware.

            // pt = dot(p - p0, p1 - p0) / |p1 - p0|^2
            let dpx = px - F32x::splat(p0[0]);
            let dpy = py - F32x::splat(p0[1]);
            let d1x = F32x::splat((p1 - p0)[0]);
            let d1y = F32x::splat((p1 - p0)[1]);
            let pt = (dpx*d1x + dpy*d1y) / (d1x*d1x + d1y*d1y);

            let pt = pt.clamp(F32x::ZERO(), F32x::ONE());

            let sr =  (F32x::ONE() - pt)*color_0[0] + pt*color_1[0];
            let sg =  (F32x::ONE() - pt)*color_0[1] + pt*color_1[1];
            let sb =  (F32x::ONE() - pt)*color_0[2] + pt*color_1[2];
            let sa = ((F32x::ONE() - pt)*color_0[3] + pt*color_1[3]) * coverage * opacity;

            let [tr, tg, tb, ta] = target[p];

            let one = F32x::splat(1.0);
            target[p] = [
                sa*sr + (one - sa)*tr,
                sa*sg + (one - sa)*tg,
                sa*sb + (one - sa)*tb,
                sa    + (one - sa)*ta,
            ];

            px += F32x::splat(n as f32);
        }

        py += F32x::ONE();
    }
}

/// - input pre-multiplied alpha: no.
pub fn fill_mask_linear_gradient_n<const N: usize>(
    p0: F32x2,
    p1: F32x2,
    stops: &[GradientStopF32],
    opacity: f32,
    mask: &Img<f32>,
    offset: U32x2,
    target: &mut ImgMut<[F32x<N>; 4]>)
where (): SimdLanes<N>
{
    let n = N as u32;

    let stop_0 = stops[0];
    let stop_n = stops[stops.len() - 1];

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

    let px0 = F32x::from_array(core::array::from_fn(|i| i as f32 + 0.5));

    let mut py = F32x::splat(0.5);

    for y in begin.y() .. end.y() {
        let mut px = px0;

        for u in u0..u1 {
            let x = u * n;
            let mask_x = (x - begin.x()) as usize;
            let mask_y = (y - begin.y()) as usize;

            let coverage = F32x::from_array(mask.read_n(mask_x, mask_y));

            let p = (u as usize, y as usize);

            if coverage.lt(F32x::splat(0.5/255.0)).all() {
                px += F32x::splat(n as f32);
                continue;
            }

            // pt = dot(p - p0, p1 - p0) / |p1 - p0|^2
            let dpx = px - F32x::splat(p0[0]);
            let dpy = py - F32x::splat(p0[1]);
            let d1x = F32x::splat((p1 - p0)[0]);
            let d1y = F32x::splat((p1 - p0)[1]);
            let pt = (dpx*d1x + dpy*d1y) / (d1x*d1x + d1y*d1y);


            let (mut sr, mut sg, mut sb, mut sa);

            let le_0 = pt.le(F32x::splat(stop_0.offset));
            let ge_n = pt.ge(F32x::splat(stop_n.offset));

            if le_0.all() {
                sr = F32x::splat(stop_0.color[0]);
                sg = F32x::splat(stop_0.color[1]);
                sb = F32x::splat(stop_0.color[2]);
                sa = F32x::splat(stop_0.color[3]);
            }
            else if ge_n.all() {
                sr = F32x::splat(stop_n.color[0]);
                sg = F32x::splat(stop_n.color[1]);
                sb = F32x::splat(stop_n.color[2]);
                sa = F32x::splat(stop_n.color[3]);
            }
            else {
                debug_assert!(stops.len() > 1);

                // handle ge_n case.
                sr = F32x::splat(stop_n.color[0]);
                sg = F32x::splat(stop_n.color[1]);
                sb = F32x::splat(stop_n.color[2]);
                sa = F32x::splat(stop_n.color[3]);

                let mut has_color = ge_n;

                for i in 0..stops.len() - 1 {
                    let curr = stops[i];
                    let next = stops[i + 1];

                    let lt_next = pt.lt(F32x::splat(next.offset));
                    let was_new = !has_color & lt_next;

                    if was_new.any() {
                        let scale = 1.0.safe_div(next.offset - curr.offset, 1_000_000.0);

                        let t = (pt - F32x::splat(curr.offset)) * scale;
                        let t = t.clamp(F32x::ZERO(), F32x::ONE());

                        let r = (F32x::ONE() - t)*curr.color[0] + t*next.color[0];
                        let g = (F32x::ONE() - t)*curr.color[1] + t*next.color[1];
                        let b = (F32x::ONE() - t)*curr.color[2] + t*next.color[2];
                        let a = (F32x::ONE() - t)*curr.color[3] + t*next.color[3];

                        sr = was_new.select(r, sr);
                        sg = was_new.select(g, sg);
                        sb = was_new.select(b, sb);
                        sa = was_new.select(a, sa);

                        has_color |= was_new;
                        if has_color.all() {
                            break;
                        }
                    }
                }

                debug_assert!(has_color.all());
            }

            let sa = sa * coverage * opacity;


            let [tr, tg, tb, ta] = target[p];

            let one = F32x::splat(1.0);
            target[p] = [
                sa*sr + (one - sa)*tr,
                sa*sg + (one - sa)*tg,
                sa*sb + (one - sa)*tb,
                sa    + (one - sa)*ta,
            ];

            px += F32x::splat(n as f32);
        }

        py += F32x::ONE();
    }
}


/// - input pre-multiplied alpha: no.
pub fn fill_mask_radial_gradient_2<const N: usize>(
    raster_origin: F32x2,
    inv_tfx: Transform,
    inv_grad_tfx: Transform,
    gradient: &RadialGradient,
    stop_0: GradientStopF32,
    stop_1: GradientStopF32,
    opacity: f32,
    mask: &Img<f32>,
    offset: U32x2,
    target: &mut ImgMut<[F32x<N>; 4]>)
where (): SimdLanes<N>
{
    let n = N as u32;

    let GradientStopF32 { color: color_0, offset: offset_0 } = stop_0;
    let GradientStopF32 { color: color_1, offset: offset_1 } = stop_1;

    let cp = gradient.cp;
    let cr = gradient.cr;
    let fp = gradient.fp;
    let fr = gradient.fr;

    let step_scale = 1.0.safe_div(offset_1 - offset_0, 1_000_000.0);

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

    let start = (inv_grad_tfx * inv_tfx) * (raster_origin + F32x2::new(0.5, 0.5));
    let x_hat = (inv_grad_tfx * inv_tfx).mul_normal(F32x2::new(1.0, 0.0));
    let y_hat = (inv_grad_tfx * inv_tfx).mul_normal(F32x2::new(0.0, 1.0));

    let x_offsets_x = F32x::from_array(core::array::from_fn(|i| i as f32)) * x_hat[0];
    let x_offsets_y = F32x::from_array(core::array::from_fn(|i| i as f32)) * x_hat[1];
    //let x_offsets_x = F32x::new((0.0*x_hat)[0], (1.0*x_hat)[0], (2.0*x_hat)[0], (3.0*x_hat)[0]);
    //let x_offsets_y = F32x::new((0.0*x_hat)[1], (1.0*x_hat)[1], (2.0*x_hat)[1], (3.0*x_hat)[1]);

    let mut pp = start;

    for y in begin.y() .. end.y() {
        let mut px = F32x::splat(pp[0]) + x_offsets_x;
        let mut py = F32x::splat(pp[1]) + x_offsets_y;

        for u in u0..u1 {
            let x = u * n;
            let mask_x = (x - begin.x()) as usize;
            let mask_y = (y - begin.y()) as usize;

            let coverage = F32x::from_array(mask.read_n(mask_x, mask_y));

            let p = (u as usize, y as usize);

            if coverage.lt(F32x::splat(0.5/255.0)).all() {
                px += F32x::splat(n as f32 * x_hat[0]);
                py += F32x::splat(n as f32 * x_hat[1]);
                continue;
            }

            let d1x = px - F32x::splat(fp[0]);
            let d1y = py - F32x::splat(fp[1]);

            let d2x = F32x::splat((fp - cp)[0]);
            let d2y = F32x::splat((fp - cp)[1]);

            // k = (-(d1 d2)) / (d1 d1) + sqrt(((d1 d2) / (d1 d1))² + (cr² - d2 d2) / (d1 d1))
            let d11 = d1x*d1x + d1y*d1y;
            let d12 = d1x*d2x + d1y*d2y;
            let d22 = d2x*d2x + d2y*d2y;
            let discr = (d12/d11)*(d12/d11) + (F32x::splat(cr*cr) - d22)/d11;
            // @todo: handle negatives.
            let discr = discr.at_least(F32x::ZERO());
            let k = -(d12/d11) + discr.sqrt();

            // t = (Length(p - fp) - fr) / (k*Length((p-fp)) - fr)
            let l = d11.sqrt();
            let fr = F32x::splat(fr);
            let pt = (l - fr) / (k*l - fr);

            let pt = (pt - F32x::splat(offset_0)) * step_scale;

            let pt = pt.clamp(F32x::ZERO(), F32x::ONE());

            let sr =  (F32x::ONE() - pt)*color_0[0] + pt*color_1[0];
            let sg =  (F32x::ONE() - pt)*color_0[1] + pt*color_1[1];
            let sb =  (F32x::ONE() - pt)*color_0[2] + pt*color_1[2];
            let sa = ((F32x::ONE() - pt)*color_0[3] + pt*color_1[3]) * coverage * opacity;

            let [tr, tg, tb, ta] = target[p];

            let one = F32x::splat(1.0);
            target[p] = [
                sa*sr + (one - sa)*tr,
                sa*sg + (one - sa)*tg,
                sa*sb + (one - sa)*tb,
                sa    + (one - sa)*ta,
            ];

            px += F32x::splat(n as f32 * x_hat[0]);
            py += F32x::splat(n as f32 * x_hat[1]);
        }

        pp += y_hat;
    }
}

/// - input pre-multiplied alpha: no.
pub fn fill_mask_radial_gradient_n<const N: usize>(
    raster_origin: F32x2,
    inv_tfx: Transform,
    inv_grad_tfx: Transform,
    gradient: &RadialGradient,
    stops: &[GradientStopF32],
    opacity: f32,
    mask: &Img<f32>,
    offset: U32x2,
    target: &mut ImgMut<[F32x<N>; 4]>)
where (): SimdLanes<N>
{
    let n = 4;

    let stop_0 = stops[0];
    let stop_n = stops[stops.len() - 1];

    let cp = gradient.cp;
    let cr = gradient.cr;
    let fp = gradient.fp;
    let fr = gradient.fr;

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

    let start = (inv_grad_tfx * inv_tfx) * (raster_origin + F32x2::new(0.5, 0.5));
    let x_hat = (inv_grad_tfx * inv_tfx).mul_normal(F32x2::new(1.0, 0.0));
    let y_hat = (inv_grad_tfx * inv_tfx).mul_normal(F32x2::new(0.0, 1.0));

    let x_offsets_x = F32x::from_array(core::array::from_fn(|i| i as f32)) * x_hat[0];
    let x_offsets_y = F32x::from_array(core::array::from_fn(|i| i as f32)) * x_hat[1];
    //let x_offsets_x = F32x::new((0.0*x_hat)[0], (1.0*x_hat)[0], (2.0*x_hat)[0], (3.0*x_hat)[0]);
    //let x_offsets_y = F32x::new((0.0*x_hat)[1], (1.0*x_hat)[1], (2.0*x_hat)[1], (3.0*x_hat)[1]);

    let mut pp = start;

    for y in begin.y() .. end.y() {
        let mut px = F32x::splat(pp[0]) + x_offsets_x;
        let mut py = F32x::splat(pp[1]) + x_offsets_y;

        for u in u0..u1 {
            let x = u * n;
            let mask_x = (x - begin.x()) as usize;
            let mask_y = (y - begin.y()) as usize;

            let coverage = F32x::from_array(mask.read_n(mask_x, mask_y));

            let p = (u as usize, y as usize);

            if coverage.lt(F32x::splat(0.5/255.0)).all() {
                px += F32x::splat(n as f32 * x_hat[0]);
                py += F32x::splat(n as f32 * x_hat[1]);
                continue;
            }

            let d1x = px - F32x::splat(fp[0]);
            let d1y = py - F32x::splat(fp[1]);

            let d2x = F32x::splat((fp - cp)[0]);
            let d2y = F32x::splat((fp - cp)[1]);

            // k = (-(d1 d2)) / (d1 d1) + sqrt(((d1 d2) / (d1 d1))² + (cr² - d2 d2) / (d1 d1))
            let d11 = d1x*d1x + d1y*d1y;
            let d12 = d1x*d2x + d1y*d2y;
            let d22 = d2x*d2x + d2y*d2y;
            let discr = (d12/d11)*(d12/d11) + (F32x::splat(cr*cr) - d22)/d11;
            // @todo: handle negatives.
            let discr = discr.at_least(F32x::ZERO());
            let k = -(d12/d11) + discr.sqrt();

            // t = (Length(p - fp) - fr) / (k*Length((p-fp)) - fr)
            let l = d11.sqrt();
            let fr = F32x::splat(fr);
            let pt = (l - fr) / (k*l - fr);


            let (mut sr, mut sg, mut sb, mut sa);

            let le_0 = pt.le(F32x::splat(stop_0.offset));
            let ge_n = pt.ge(F32x::splat(stop_n.offset));

            if le_0.all() {
                sr = F32x::splat(stop_0.color[0]);
                sg = F32x::splat(stop_0.color[1]);
                sb = F32x::splat(stop_0.color[2]);
                sa = F32x::splat(stop_0.color[3]);
            }
            else if ge_n.all() {
                sr = F32x::splat(stop_n.color[0]);
                sg = F32x::splat(stop_n.color[1]);
                sb = F32x::splat(stop_n.color[2]);
                sa = F32x::splat(stop_n.color[3]);
            }
            else {
                debug_assert!(stops.len() > 1);

                // handle ge_n case.
                sr = F32x::splat(stop_n.color[0]);
                sg = F32x::splat(stop_n.color[1]);
                sb = F32x::splat(stop_n.color[2]);
                sa = F32x::splat(stop_n.color[3]);

                let mut has_color = ge_n;

                for i in 0..stops.len() - 1 {
                    let curr = stops[i];
                    let next = stops[i + 1];

                    let lt_next = pt.lt(F32x::splat(next.offset));
                    let was_new = !has_color & lt_next;

                    if was_new.any() {
                        let scale = 1.0.safe_div(next.offset - curr.offset, 1_000_000.0);

                        let t = (pt - F32x::splat(curr.offset)) * scale;
                        let t = t.clamp(F32x::ZERO(), F32x::ONE());

                        let r = (F32x::ONE() - t)*curr.color[0] + t*next.color[0];
                        let g = (F32x::ONE() - t)*curr.color[1] + t*next.color[1];
                        let b = (F32x::ONE() - t)*curr.color[2] + t*next.color[2];
                        let a = (F32x::ONE() - t)*curr.color[3] + t*next.color[3];

                        sr = was_new.select(r, sr);
                        sg = was_new.select(g, sg);
                        sb = was_new.select(b, sb);
                        sa = was_new.select(a, sa);

                        has_color |= was_new;
                        if has_color.all() {
                            break;
                        }
                    }
                }

                debug_assert!(has_color.all());
            }

            let sa = sa * coverage * opacity;


            let [tr, tg, tb, ta] = target[p];

            let one = F32x::splat(1.0);
            target[p] = [
                sa*sr + (one - sa)*tr,
                sa*sg + (one - sa)*tg,
                sa*sb + (one - sa)*tb,
                sa    + (one - sa)*ta,
            ];

            px += F32x::splat(n as f32 * x_hat[0]);
            py += F32x::splat(n as f32 * x_hat[1]);
        }

        pp += y_hat;
    }
}

