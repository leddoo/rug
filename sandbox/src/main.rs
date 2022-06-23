#![feature(portable_simd)]

mod win32;

use rug::*;

fn main() {
    if 0==1 {
        let svg = {
            //let file = include_bytes!(r"D:\dev\vg-inputs\svg\tiger.svg");
            let file = include_bytes!(r"D:\dev\vg-inputs\svg\paris-30k.svg");
            //let file = include_bytes!(r"D:\dev\vg-inputs\svg\hawaii.svg");
            parse_xml(core::str::from_utf8(file).unwrap())
        };

        let tfx = Transform::scale(1.0);

        let iters = 50;
        let t0 = std::time::Instant::now();

        let mut image = vec![];
        for _ in 0..iters {
            //_render_svg(&svg, 2560, 1440, tfx, &mut image);
            //_render_svg(&svg, 2048, 2048, tfx, &mut image);
            //_render_svg(&svg, 1920, 1080, tfx, &mut image);
            _render_svg(&svg, 1024, 1024, tfx, &mut image);
            //_render_svg(&svg, 512, 512, tfx, &mut image);
            //break;
        }

        println!("{:?}", t0.elapsed() / iters);

        win32::exit();
    }

    win32::run(program);
}

fn program() {
    let svg = {
        //let file = include_bytes!(r"D:\dev\vg-inputs\svg\tiger.svg");
        let file = include_bytes!(r"D:\dev\vg-inputs\svg\paris-30k.svg");
        //let file = include_bytes!(r"D:\dev\vg-inputs\svg\hawaii.svg");
        //let file = include_bytes!(r"D:\dev\vg-inputs\svg\boston.svg");
        //let file = include_bytes!(r"D:\dev\vg-inputs\svg\paper-1.svg");
        //let file = include_bytes!(r"D:\dev\vg-inputs\svg\embrace.svg");
        //let file = include_bytes!(r"D:\dev\vg-inputs\svg\reschart.svg");
        parse_xml(core::str::from_utf8(file).unwrap())
    };

    let mut zoom = 0;
    let mut scale = 1.0;
    let mut offset = F32x2::ZERO;
    let mut panning = false;
    let mut mouse = F32x2::ZERO;

    let window = win32::Window::new();
    let mut old_size = (0, 0);
    loop {
        let mut event = win32::next_event_timeout(std::time::Duration::from_secs(2));
        while let Some(e) = event {
            use win32::Event::*;
            match e {
                Close (_) => { win32::exit(); },
                Paint (_) => { old_size = (0, 0) },

                MouseMove (_, x, y) => {
                    let size = window.size();
                    let new_mouse = F32x2::new(x as f32, (size.1 - 1 - y) as f32);
                    let delta = new_mouse - mouse;
                    mouse = new_mouse;
                    if panning {
                        offset += delta;
                        old_size = (0, 0);
                    }
                }

                MouseDown (_, _, _, which) => {
                    if let win32::MouseButton::Right = which {
                        panning = true;
                    }
                }

                MouseUp (_, _, _, which) => {
                    if let win32::MouseButton::Right = which {
                        panning = false;
                    }
                }

                MouseWheel (_, delta) => {
                    zoom += delta;
                    let new_scale = libm::expf(zoom as f32 * 0.1);

                    offset = mouse - (new_scale/scale).mul(mouse - offset);
                    scale = new_scale;

                    old_size = (0, 0);
                }

                _ => (),
            }
            event = win32::peek_event();
        }

        let size = window.size();
        if size != old_size {
            old_size = size;

            let (w, h) = size;
            let tfx = Transform::translate(offset) * Transform::scale(scale);
            let buffer = render_svg(&svg, w, h, tfx);
            window.fill_pixels(&buffer, 0, 0, w, h);
        }
    }
}


#[derive(Clone, Copy)]
enum SvgCommand<'p> {
    Fill (PathRef<'p>, F32x4),
    Stroke (PathRef<'p>, F32x4, F32),
}

struct Svg<'p> {
    commands: Vec<SvgCommand<'p>>,
    aabbs: Vec<Rect>,
}


#[inline(never)]
fn target_to_argb_at(target: &Target, w: usize, h: usize, output: &mut Vec<u32>, start: usize, stride: usize) {
    const N: usize = Target::simd_width();

    for y in 0..h {
        let offset = start + y*stride;

        let y = (h - 1) - y;

        for x in 0..(w / N) {
            let rgba = target[(x, y)];
            let argb = argb_u8x_pack(rgba);
            for dx in 0..N {
                output[offset + N*x + dx] = argb.as_array()[dx];
            }
        }

        let rem = w % N;
        if rem > 0 {
            let x = w/N;
            let rgba = target[(x, y)];
            let argb = argb_u8x_pack(rgba);
            for dx in 0..rem {
                output[offset + N*x + dx] = argb.as_array()[dx];
            }
        }
    }
}


fn parse_xml(xml: &str) -> Svg<'static> {
    let xml = roxmltree::Document::parse(xml).unwrap();

    let root = xml.root();
    let svg = root.children().next().unwrap();

    let mut result = Svg { aabbs: vec![], commands: vec![] };

    for child in svg.children() {
        render(&mut result, child);
    }

    fn render(result: &mut Svg, node: roxmltree::Node) -> Option<()> {
        if !node.is_element() {
            return None;
        }

        let tag_name = node.tag_name();

        match tag_name.name() {
            "g" => {
                for child in node.children() {
                    render(result, child);
                }
            },

            "defs" => {
                // todo.
            },

            "path" => {
                use core::str::FromStr;

                use svgtypes::PathSegment::*;

                fn to_vec(x: f64, y: f64) -> F32x2 {
                    F32x2::new(x as f32, y as f32)
                }

                let mut pb = PathBuilder::new();
                for curve in svgtypes::PathParser::from(node.attribute("d").unwrap()) {
                    match curve.unwrap() {
                        MoveTo { abs, x, y } => {
                            assert!(abs);
                            pb.move_to(to_vec(x, y));
                        },

                        LineTo { abs, x: x1, y: y1 } => {
                            assert!(abs);
                            pb.segment_to(to_vec(x1, y1));
                        },

                        Quadratic { abs, x1, y1, x: x2, y: y2 } => {
                            assert!(abs);
                            pb.quadratic_to(to_vec(x1, y1), to_vec(x2, y2));
                        },

                        CurveTo { abs, x1, y1, x2, y2, x: x3, y: y3 } => {
                            assert!(abs);
                            pb.cubic_to(to_vec(x1, y1), to_vec(x2, y2), to_vec(x3, y3));
                        },

                        ClosePath { abs } => {
                            assert!(abs);
                            pb.close();
                        },

                        _ => {
                            println!("unknown curve type");
                            return None;
                        },
                    }
                }

                let path = pb.build().leak();

                if let Some(fill) = node.attribute("fill") {
                    if let Ok(color) = svgtypes::Color::from_str(fill) {

                        let alpha =
                            if let Some(opacity) = node.attribute("fill-opacity") {
                                svgtypes::Number::from_str(opacity).unwrap().0 as f32
                            }
                            else {
                                1.0
                            };

                        let a = alpha * (color.alpha as f32 / 255.0);
                        let color = F32x4::new(
                            color.red   as f32 / 255.0,
                            color.green as f32 / 255.0,
                            color.blue  as f32 / 255.0,
                            a,
                        );

                        result.aabbs.push(path.aabb());
                        result.commands.push(SvgCommand::Fill(path, color));
                    }
                    else if fill != "none" {
                        println!("unknown fill: {}", fill);
                    }
                }

                if let Some(stroke) = node.attribute("stroke") {
                    if let Ok(color) = svgtypes::Color::from_str(stroke) {

                        let alpha =
                            if let Some(opacity) = node.attribute("stroke-opacity") {
                                svgtypes::Number::from_str(opacity).unwrap().0 as f32
                            }
                            else {
                                1.0
                            };

                        let width =
                            if let Some(width) = node.attribute("stroke-width") {
                                svgtypes::Number::from_str(width).unwrap().0 as f32
                            }
                            else {
                                1.0
                            };

                        let a = alpha * (color.alpha as f32 / 255.0);
                        let color = F32x4::new(
                            color.red   as f32 / 255.0,
                            color.green as f32 / 255.0,
                            color.blue  as f32 / 255.0,
                            a,
                        );

                        // TODO: technically this isn't the aabb of the _stroked_ path.
                        result.aabbs.push(path.aabb());
                        result.commands.push(SvgCommand::Stroke(path, color, width));
                    }
                    else if stroke != "none" {
                        println!("unknown stroke: {}", stroke);
                    }
                }
            },

            _ => {
                println!("unknown tag: {}", tag_name.name());
            },
        }

        Some(())
    }

    assert!(result.aabbs.len() == result.commands.len());
    result
}


fn rasterize<F: FnOnce(F32x2, &mut Rasterizer)>(tile: Rect, aabb: Rect, f: F) -> Option<(U32x2, Mask<'static>)> {
    let aabb = unsafe { aabb.clamp_to(tile).round_inclusive_unck() };

    const N: usize = Target::simd_width();
    const NF32: f32 = N as F32;

    let x0 = floor_fast(aabb.min.x() / NF32) * NF32;
    let x1 = ceil_fast(aabb.max.x() / NF32)  * NF32;

    let mask_w = (x1 - x0)     as U32;
    let mask_h = aabb.height() as U32;
    if mask_w == 0 || mask_h == 0 {
        return None;
    }

    let p0 = F32x2::new(x0, aabb.min.y());

    let mut r = Rasterizer::new(mask_w, mask_h);
    f(p0, &mut r);

    let offset = (p0 - tile.min).to_i32().as_u32();
    Some((offset, r.accumulate()))
}

fn rasterize_fill(tile: Rect, path: PathRef, tfx: Transform) -> Option<(U32x2, Mask<'static>)> {
    rasterize(tile, tfx.aabb_transform(path.aabb()), |p0, r| {
        let mut tfx = tfx;
        tfx.columns[2] -= p0;
        r.fill_path_tfx(path, tfx)
    })
}

fn rasterize_fill_soa(tile: Rect, path: &SoaPath) -> Option<(U32x2, Mask<'static>)> {
    rasterize(tile, path.aabb, |p0, r| r.fill_soa_path(path, p0))
}


fn render_svg(svg: &Svg, w: u32, h: u32, tfx: Transform) -> Vec<u32> {
    let mut output = vec![];
    _render_svg(svg, w, h, tfx, &mut output);
    output
}

#[inline(never)]
fn _render_svg(svg: &Svg, w: u32, h: u32, tfx: Transform, output: &mut Vec<u32>) {
    output.clear();
    output.reserve((w*h) as usize);
    unsafe { output.set_len((w*h) as usize) };

    let tile_size = 160;
    let mut tile_target = Target::new(tile_size, tile_size);

    let tiles_x = (w + tile_size - 1) / tile_size;
    let tiles_y = (h + tile_size - 1) / tile_size;
    let tile_count = (tiles_x * tiles_y) as usize;

    let mut paths = 0;
    let mut fragments = 0;

    let t0 = std::time::Instant::now();

    let path_count = svg.commands.len();
    let visible_count = (path_count + 63)/64;
    let mut visible = vec![0; visible_count*tile_count];

    let mut strokes = vec![None; svg.commands.len()];

    for (command_index, command) in svg.commands.iter().enumerate() {

        fn fill_visible(visible: &mut Vec<u64>, visible_count: usize, path_index: usize, path_aabb: Rect,
            tile_size: u32, tiles_x: u32, tiles_y: u32
        ) {
            let path_bit = 1 << (path_index as u64 % 64);

            let tiles_end = F32x2::new(tiles_x as F32, tiles_y as F32);

            let tile_size = F32x2::splat(tile_size as F32);
            let rect = rect(path_aabb.min / tile_size, path_aabb.max / tile_size);
            let rect = unsafe { rect.round_inclusive_unck() };
            let begin = unsafe { rect.min.clamp(F32x2::ZERO, tiles_end).to_i32_unck().as_u32() };
            let end   = unsafe { rect.max.clamp(F32x2::ZERO, tiles_end).to_i32_unck().as_u32() };

            for y in begin[1]..end[1] {
                for x in begin[0]..end[0] {
                    let base = (y*tiles_x + x) as usize * visible_count;
                    visible[base + path_index/64] |= path_bit;
                }
            }
        }

        match command {
            SvgCommand::Fill (_path, _color) => {
                let aabb = tfx.aabb_transform(svg.aabbs[command_index]);
                fill_visible(&mut visible, visible_count, command_index, aabb, tile_size, tiles_x, tiles_y);
            },

            SvgCommand::Stroke (path, _color, width) => {
                let mut path = stroke_path(path, *width);
                path.transform(tfx);
                let aabb = path.aabb;
                strokes[command_index] = Some(path);
                fill_visible(&mut visible, visible_count, command_index, aabb, tile_size, tiles_x, tiles_y);
            },
        }
    }

    for ty in 0..tiles_y {
        let tile_y0 = ty*tile_size;
        let tile_y1 = (tile_y0 + tile_size).min(h);

        for tx in 0..tiles_x {
            let tile_x0 = tx*tile_size;
            let tile_x1 = (tile_x0 + tile_size).min(w);

            let base = (ty*tiles_x + tx) as usize * visible_count;
            let visible = &visible[base .. base + visible_count];

            let tile = rect(
                F32x2::new(tile_x0 as f32, tile_y0 as f32),
                F32x2::new(tile_x1 as f32, tile_y1 as f32),
            );

            //tile_target.clear(F32x4::new(1.0, 0.0, 1.0, 1.0));
            tile_target.clear(F32x4::new(15.0/255.0, 20.0/255.0, 25.0/255.0, 1.0));

            let mut base_index = 0;
            for visible in visible {
                let mut visible = *visible;

                while visible != 0 {
                    let offset = visible.trailing_zeros();
                    let command_index = base_index + offset as usize;
                    visible &= !(1 << offset);

                    match &svg.commands[command_index] {
                        SvgCommand::Fill (path, color) => {
                            if let Some((offset, mask)) = rasterize_fill(tile, path, tfx) {
                                paths += 1;
                                fragments += (mask.width() * mask.height()) as usize;
                                fill_mask(&mut tile_target, offset, &mask, *color);
                            }
                        },

                        SvgCommand::Stroke (_path, color, _width) => {
                            let path = strokes[command_index].as_ref().unwrap();
                            if let Some((offset, mask)) = rasterize_fill_soa(tile, path) {
                                paths += 1;
                                fragments += (mask.width() * mask.height()) as usize;
                                fill_mask(&mut tile_target, offset, &mask, *color);
                            }
                        },
                    }
                }

                base_index += 64;
            }

            let stride = w;
            let start = ((h - tile_y1)*stride + tile_x0) as usize;
            target_to_argb_at(&tile_target, (tile_x1 - tile_x0) as usize, (tile_y1 - tile_y0) as usize, output, start, stride as usize);
        }
    }


    if 0==1 {
        let dt = t0.elapsed();
        let size = (tile_target.stride() * tile_target.height() as usize) * core::mem::size_of::<[F32x<{Target::simd_width()}>; 4]>();
        let pixels = w*h;
        let dt_path = dt.as_secs_f32() * 1000.0 * 1000.0 / paths as f32;
        let dt_frag = dt.as_secs_f32() * 1000.0 * 1000.0 * 1000.0 / fragments as f32;
        let dt_pix = dt.as_secs_f32() * 1000.0 * 1000.0 * 1000.0 / pixels as f32;

        print!("{}x{}, {} kiB\n", w, h, size / 1024);
        print!("{} paths in {:.2?}\n", paths, dt);
        print!("{:.2?}us per path\n", dt_path);
        print!("{:.2?}ns per fragment\n", dt_frag);
        print!("{:.2?}ns per pixel\n", dt_pix);
        print!("{} fragments\n", fragments);
        print!("{} pixels\n", pixels);
        print!("{:.2} frags/pixel\n", fragments as f32 / pixels as f32);
        print!("{:.2} frags/path\n", fragments as f32 / paths as f32);
        println!();
    }
}

