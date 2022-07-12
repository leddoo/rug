use core::sync::atomic::{AtomicU64, Ordering};

use sti::simd::*;

use crate::{
    alloc::*,
    geometry::{rect, Rect, Transform},
    image::*,
    path::*,
    rasterizer::*,
    stroke::*,
};


pub enum Command<A: Alloc> {
    FillPathSolid {
        path:  Path<A>,
        color: u32,
        rule:  FillRule,
    },
    StrokePathSolid {
        path:  Path<A>,
        color: u32,
        width: f32,
        cap:   CapStyle,
        join:  JoinStyle,
    },
}


impl<A: Alloc> Command<A> {
    #[inline(always)]
    pub fn aabb(&self, tfx: Transform, stroke: &Option<SoaPath>) -> Rect {
        tfx.aabb_transform(match self {
            Command::FillPathSolid { path, color: _, rule: _ } =>
                path.aabb(),

            Command::StrokePathSolid { path: _, color: _, width: _, cap: _, join: _ } =>
                stroke.as_ref().unwrap().aabb,
        })
    }

    #[inline(always)]
    pub fn rasterize<B: Alloc>(&self, tfx: Transform, r: &mut Rasterizer<B>, stroke: &Option<SoaPath>) {
        match self {
            Command::FillPathSolid { path, color: _, rule: _ } => {
                r.fill_path(path, tfx);
            }

            Command::StrokePathSolid { path: _, color: _, width: _, cap: _, join: _ } => {
                r.fill_soa_path(stroke.as_ref().unwrap(), tfx);
            }
        }
    }

    #[inline(always)]
    pub fn color(&self) -> u32 {
        match self {
            Command::FillPathSolid { path: _, color, rule: _ } => *color,
            Command::StrokePathSolid { path: _, color, width: _, cap: _, join: _ } => *color,
        }
    }
}


/// - commands[i] ~ masks[index_offset + i], strokes[index_offset + i]
/// - strokes must be Some for all stroke commands
pub fn fill_masks<A: Alloc>(
    masks: &[CommandMask],
    commands: &[Command<A>],
    index_offset: usize,
    tile_size: u32,
    tile_counts: U32x2,
    tfx: Transform,
    strokes: &[Option<SoaPath>],
) {
    let tiles_x = tile_counts.x();
    let tile_size = tile_size as f32;
    let tile_counts = tile_counts.as_i32().to_f32();

    for (index, command) in commands.iter().enumerate() {
        let index = index + index_offset;
        let aabb = command.aabb(tfx, &strokes[index]);
        fill_visible(&masks, index, aabb, tile_size, tile_counts, tiles_x);
    }


    #[inline(always)]
    fn fill_visible(masks: &[CommandMask], cmd_index: usize, cmd_aabb: Rect,
        tile_size: f32, tile_counts: F32x2, tiles_x: u32,
    ) {
        let rect = rect(cmd_aabb.min.div(tile_size), cmd_aabb.max.div(tile_size));
        let rect = unsafe { rect.round_inclusive_unck() };
        let begin = unsafe { rect.min.clamp(F32x2::ZERO, tile_counts).to_i32_unck().as_u32() };
        let end   = unsafe { rect.max.clamp(F32x2::ZERO, tile_counts).to_i32_unck().as_u32() };

        for y in begin[1]..end[1] {
            for x in begin[0]..end[0] {
                masks[(y*tiles_x + x) as usize].add(cmd_index);
            }
        }
    }
}


pub struct Tile<'img, T: Copy> {
    pub img:  ImgMut<'img, T>,
    pub rect: Rect,
}

impl<'img, const N: usize> Tile<'img, [F32x<N>; 4]> where LaneCount<N>: SupportedLaneCount {
    #[inline(always)]
    pub fn new(img: ImgMut<'img, [F32x<N>; 4]>, rect: Rect) -> Self {
        Self { img, rect }
    }

    /// - stroke must be Some if command is a stroke command.
    #[inline(never)]
    pub fn execute<A: Alloc>(
        &mut self,
        command: &Command<A>,
        tfx: Transform,
        stroke: &Option<SoaPath>,
    ) {
        let (raster_size, raster_origin, blit_offset) =
            <Rasterizer<GlobalAlloc>>::rect_for(
                command.aabb(tfx, stroke),
                self.rect,
                N as u32);

        if raster_size.lanes_eq(U32x2::ZERO).any() {
            return;
        }

        let mut r = Rasterizer::new(raster_size);
        let mut tfx = tfx;
        tfx.columns[2] -= raster_origin;
        command.rasterize(tfx, &mut r, stroke);

        let color = command.color();

        let mask = r.accumulate();
        fill_mask(&mut self.img, blit_offset, mask.view(), argb_unpack(color));
    }
}


pub fn fill_mask<const N: usize>(
    target: &mut ImgMut<[F32x<N>; 4]>,
    offset: U32x2,
    mask: Img<f32>,
    color: F32x4)
    where LaneCount<N>: SupportedLaneCount
{
    let n = N as u32;

    let size = target.size() * U32x2::new(n, 1);

    let begin = offset;
    let end   = (offset + mask.size()).min(size);
    if begin.lanes_eq(end).any() {
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

            if coverage.lanes_lt(F32x::splat(0.5/255.0)).all() {
                continue;
            }
            if color[3] == 1.0 && coverage.lanes_gt(F32x::splat(254.5/255.0)).all() {
                target[p] = [
                    F32x::splat(color[0]),
                    F32x::splat(color[1]),
                    F32x::splat(color[2]),
                    F32x::splat(1.0),
                ];
                continue;
            }

            let [tr, tg, tb, ta] = target[p];

            let sr = F32x::splat(color[0]);
            let sg = F32x::splat(color[1]);
            let sb = F32x::splat(color[2]);
            let sa = F32x::splat(color[3]) * coverage;

            let one = F32x::splat(1.0);
            target[p] = [
                sa*sr + (one - sa)*ta*tr,
                sa*sg + (one - sa)*ta*tg,
                sa*sb + (one - sa)*ta*tb,
                sa    + (one - sa)*ta,
            ];
        }
    }
}



pub struct CommandMask {
    values: Vec<AtomicU64>,
}

impl CommandMask {
    pub fn new(size: usize) -> Self {
        let len = (size + 63) / 64;

        let mut values = vec![];
        values.reserve(len);
        for _ in 0..len {
            values.push(AtomicU64::new(0));
        }

        Self { values}
    }

    #[inline(always)]
    pub fn add_mut(&mut self, index: usize) {
        let bit = 1 << (index as u64 % 64);
        *self.values[index / 64].get_mut() |= bit;
    }

    #[inline(always)]
    pub fn add(&self, index: usize) {
        let bit = 1 << (index as u64 % 64);
        self.values[index / 64].fetch_or(bit, Ordering::Relaxed);
    }

    // not actually mut, just exclusive (to access atomic value directly).
    pub fn iter<F: FnMut(usize)>(&mut self, mut f: F) {
        let mut base = 0;

        for bits in self.values.as_mut_slice() {
            let mut bits = *bits.get_mut();
            while bits != 0 {
                let offset = bits.trailing_zeros();
                let command_index = base + offset as usize;
                bits &= !(1 << offset);

                f(command_index);
            }

            base += 64;
        }
    }
}

