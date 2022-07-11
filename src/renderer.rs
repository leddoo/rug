use core::sync::atomic::{AtomicU64, Ordering};

use sti::simd::*;

use crate::{
    alloc::*,
    float::*,
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
        match command {
            Command::FillPathSolid { path, color: _, rule: _ } => {
                let aabb = tfx.aabb_transform(path.aabb());
                fill_visible(&masks, index, aabb, tile_size, tile_counts, tiles_x);
            },

            Command::StrokePathSolid { path: _, color: _, width: _, cap: _, join: _ } => {
                let path = strokes[index].as_ref().unwrap();
                let aabb = tfx.aabb_transform(path.aabb);
                fill_visible(&masks, index, aabb, tile_size, tile_counts, tiles_x);
            },
        }
    }


    #[inline(always)]
    fn fill_visible(masks: &[CommandMask], cmd_index: usize, path_aabb: Rect,
        tile_size: f32, tile_counts: F32x2, tiles_x: u32,
    ) {
        let rect = rect(path_aabb.min.div(tile_size), path_aabb.max.div(tile_size));
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

impl<'img, const N: usize> Tile<'img, [F32x<N>; 4]> where LaneCount<N>: SupportedLaneCount
{
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
        // look, it ain't pretty, but it works.
        match command {
            Command::FillPathSolid { path, color, rule: _ } => {
                let r = rasterize(self.rect, path.aabb(), N as f32, tfx, |tfx, r| {
                    r.fill_path(path, tfx)
                });

                if let Some((offset, mask)) = r {
                    fill_mask(&mut self.img, offset, &mask, argb_unpack(*color));
                }
            },

            Command::StrokePathSolid { path: _, color, width: _, cap: _, join: _ } => {
                let path = stroke.as_ref().unwrap();
                let r = rasterize(self.rect, path.aabb, N as f32, tfx, |tfx, r| {
                    r.fill_soa_path(path, tfx)
                });

                if let Some((offset, mask)) = r {
                    fill_mask(&mut self.img, offset, &mask, argb_unpack(*color));
                }
            },
        }

        fn rasterize<F: FnOnce(Transform, &mut Rasterizer)>(tile: Rect, aabb: Rect, n: f32, tfx: Transform, f: F)
            -> Option<(U32x2, Mask<'static>)>
        {
            let aabb = tfx.aabb_transform(aabb);
            let aabb = unsafe { aabb.clamp_to(tile).round_inclusive_unck() };

            let x0 = floor_fast(aabb.min.x() / n) * n;
            let x1 = ceil_fast(aabb.max.x() / n)  * n;

            let mask_w = (x1 - x0)     as u32;
            let mask_h = aabb.height() as u32;
            if mask_w == 0 || mask_h == 0 {
                return None;
            }

            let p0 = F32x2::new(x0, aabb.min.y());
            let mut tfx = tfx;
            tfx.columns[2] -= p0;

            let mut r = Rasterizer::new(mask_w, mask_h);
            f(tfx, &mut r);

            let offset = (p0 - tile.min).to_i32().as_u32();
            Some((offset, r.accumulate()))
        }
    }

}


pub fn fill_mask<const N: usize>(
    target: &mut ImgMut<[F32x<N>; 4]>,
    offset: U32x2,
    mask: &Mask,
    color: F32x4)
    where LaneCount<N>: SupportedLaneCount
{
    let n = N as u32;

    let size = target.size() * U32x2::new(n, 1);

    let begin = offset;
    let end   = (offset + mask.bounds()).min(size);
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

            let coverage = mask.read(mask_x, mask_y);

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

            let [tr, tg, tb, ta] = target[p];

            let sr = <F32x<N>>::splat(color[0]);
            let sg = <F32x<N>>::splat(color[1]);
            let sb = <F32x<N>>::splat(color[2]);
            let sa = <F32x<N>>::splat(color[3]) * coverage;

            let one = <F32x<N>>::splat(1.0);
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

