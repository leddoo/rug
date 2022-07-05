use core::sync::atomic::{AtomicU64, Ordering};

use sti::simd::*;

use crate::{
    alloc::*,
    float::*,
    geometry::{rect, Rect, Transform},
    image::*,
    path::*,
    pipeline::fill_mask,
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


pub struct CommandBuffer<A: Alloc> {
    commands: Vec<Command<A>, A>,
}

impl CommandBuffer<GlobalAlloc> {
    pub fn new() -> Self {
        CommandBuffer::new_in(GlobalAlloc)
    }
}


impl<A: Alloc> CommandBuffer<A> {
    pub fn new_in(alloc: A) -> Self {
        Self {
            commands: Vec::new_in(alloc),
        }
    }

    #[inline(always)]
    pub fn add(&mut self, command: Command<A>) {
        self.commands.push(command);
    }


    pub fn execute(&self, target: &mut RenderTarget, tfx: Transform) {
        let [w, h] = target.size.as_array();
        let tile_size = 160;

        let tiles_x = (w + tile_size-1) / tile_size;
        let tiles_y = (h + tile_size-1) / tile_size;
        let tile_count = (tiles_x * tiles_y) as usize;


        let mut tile_masks = Vec::new();
        tile_masks.resize_with(tile_count, || CommandMask::new(self.commands.len()));

        let mut strokes = vec![None; self.commands.len()];

        for (command_index, command) in self.commands.iter().enumerate() {
            match command {
                Command::FillPathSolid { path, color: _, rule: _ } => {
                    let aabb = tfx.aabb_transform(path.aabb());
                    fill_visible(&tile_masks, command_index, aabb, tile_size, tiles_x, tiles_y);
                },

                Command::StrokePathSolid { path, color: _, width, cap: _, join: _ } => {
                    let mut path = stroke_path(path, *width);
                    path.transform(tfx);
                    let aabb = path.aabb;
                    strokes[command_index] = Some(path);
                    fill_visible(&tile_masks, command_index, aabb, tile_size, tiles_x, tiles_y);
                },
            }

            fn fill_visible(visible: &Vec<CommandMask>, cmd_index: usize, path_aabb: Rect,
                tile_size: u32, tiles_x: u32, tiles_y: u32
            ) {
                let tiles_end = F32x2::new(tiles_x as f32, tiles_y as f32);
                let tile_size = F32x2::splat(tile_size as f32);

                let rect = rect(path_aabb.min / tile_size, path_aabb.max / tile_size);
                let rect = unsafe { rect.round_inclusive_unck() };
                let begin = unsafe { rect.min.clamp(F32x2::ZERO, tiles_end).to_i32_unck().as_u32() };
                let end   = unsafe { rect.max.clamp(F32x2::ZERO, tiles_end).to_i32_unck().as_u32() };

                for y in begin[1]..end[1] {
                    for x in begin[0]..end[0] {
                        visible[(y*tiles_x + x) as usize].add(cmd_index);
                    }
                }
            }
        }

        let mut paths = 0;

        let mut tile_target = Target::new(tile_size, tile_size);

        for tile_index in 0..tile_count {
            let tx = tile_index as u32 % tiles_x;
            let ty = tile_index as u32 / tiles_x;

            let tile_size = U32x2::splat(tile_size);
            let tile_min = (U32x2::new(tx, ty) * tile_size).min(target.size);
            let tile_max = (tile_min + tile_size).min(target.size);

            let tile_rect = rect(tile_min.as_i32().to_f32(), tile_max.as_i32().to_f32());

            //tile_target.clear(F32x4::new(0.0, 0.0, 0.0, 1.0));
            tile_target.clear(F32x4::new(15.0/255.0, 20.0/255.0, 25.0/255.0, 1.0));

            tile_masks[tile_index].iter(|command_index| {
                match &self.commands[command_index] {
                    Command::FillPathSolid { path, color, rule: _ } => {
                        let aabb = tfx.aabb_transform(path.aabb());

                        let r = rasterize(tile_rect, aabb, |p0, r| {
                            let mut tfx = tfx;
                            tfx.columns[2] -= p0;
                            r.fill_path(path, tfx)
                        });

                        if let Some((offset, mask)) = r {
                            paths += 1;
                            fill_mask(&mut tile_target, offset, &mask, argb_unpack(*color));
                        }
                    },

                    Command::StrokePathSolid { path: _, color, width: _, cap: _, join: _ } => {
                        let path = strokes[command_index].as_ref().unwrap();

                        let r = rasterize(tile_rect, path.aabb, |p0, r| r.fill_soa_path(path, p0));

                        if let Some((offset, mask)) = r {
                            paths += 1;
                            fill_mask(&mut tile_target, offset, &mask, argb_unpack(*color));
                        }
                    },
                }

                fn rasterize<F: FnOnce(F32x2, &mut Rasterizer)>(tile: Rect, aabb: Rect, f: F)
                    -> Option<(U32x2, Mask<'static>)>
                {
                    let aabb = unsafe { aabb.clamp_to(tile).round_inclusive_unck() };

                    const N: usize = Target::simd_width();
                    const NF32: f32 = N as f32;

                    let x0 = floor_fast(aabb.min.x() / NF32) * NF32;
                    let x1 = ceil_fast(aabb.max.x() / NF32)  * NF32;

                    let mask_w = (x1 - x0)     as u32;
                    let mask_h = aabb.height() as u32;
                    if mask_w == 0 || mask_h == 0 {
                        return None;
                    }

                    let p0 = F32x2::new(x0, aabb.min.y());

                    let mut r = Rasterizer::new(mask_w, mask_h);
                    f(p0, &mut r);

                    let offset = (p0 - tile.min).to_i32().as_u32();
                    Some((offset, r.accumulate()))
                }
            });

            target.write(tile_min, tile_max - tile_min, &tile_target);
        }
    }
}


pub struct RenderTarget {
    pub data: Vec<u32>,
    pub size: U32x2,
}

impl RenderTarget {
    pub fn new(w: u32, h: u32) -> Self {
        let mut data = vec![];
        data.resize((w*h) as usize, 0);
        Self { data, size: U32x2::new(w, h) }
    }

    pub fn write(&mut self, pos: U32x2, size: U32x2, target: &Target) {
        const N: usize = Target::simd_width();

        let [x, y] = *pos.cast::<usize>().as_array();
        let [w, h] = *size.cast::<usize>().as_array();

        let stride = self.size.x() as usize;
        let start = y*stride + x;

        for y in 0..h {
            let offset = start + y*stride;

            for x in 0..(w / N) {
                let rgba = target[(x, y)];
                let argb = argb_u8x_pack(rgba);
                for dx in 0..N {
                    self.data[offset + N*x + dx] = argb.as_array()[dx];
                }
            }

            let rem = w % N;
            if rem > 0 {
                let x = w/N;
                let rgba = target[(x, y)];
                let argb = argb_u8x_pack(rgba);
                for dx in 0..rem {
                    self.data[offset + N*x + dx] = argb.as_array()[dx];
                }
            }
        }
    }
}


struct CommandMask {
    values: Vec<AtomicU64>,
}

impl CommandMask {
    fn new(size: usize) -> Self {
        let len = (size + 63) / 64;

        let mut values = vec![];
        values.reserve(len);
        for _ in 0..len {
            values.push(AtomicU64::new(0));
        }

        Self { values}
    }

    #[inline(always)]
    fn add(&self, index: usize) {
        let bit = 1 << (index as u64 % 64);
        self.values[index / 64].fetch_or(bit, Ordering::Relaxed);
    }

    // not actually mut, just exclusive (to access atomic value directly).
    fn iter<F: FnMut(usize)>(&mut self, mut f: F) {
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

