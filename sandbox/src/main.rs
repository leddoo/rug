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

        let mut image = vec![];
        for _ in 0..10 {
            _render_svg(&svg, 2560, 1440, &mut image);
            //_render_svg(&svg, 1024, 1024, &mut image);
            //_render_svg(&svg, 512, 512, &mut image);
            //break;
        }
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

    let window = win32::Window::new();
    let mut old_size = (0, 0);
    loop {
        let mut event = win32::next_event_timeout(std::time::Duration::from_secs(2));
        while let Some(e) = event {
            use win32::Event::*;
            match e {
                Close (_) => { win32::exit(); },
                Paint (_) => { old_size = (0, 0) },
                _ => (),
            }
            event = win32::peek_event();
        }

        let size = window.size();
        if size != old_size {
            old_size = size;

            let (w, h) = size;
            let buffer = render_svg(&svg, w, h);
            window.fill_pixels(&buffer, 0, 0, w, h);
        }
    }
}


enum SvgCommand<'a> {
    Fill (Path<'a>, F32x4),
    Stroke (Path<'a>, F32x4, F32),
}

struct Svg<'a> {
    commands: Vec<SvgCommand<'a>>,
}


fn target_to_argb(target: &Target) -> Vec<u32> {
    let mut output = vec![];
    _target_to_argb(target, &mut output);
    output
}

fn _target_to_argb(target: &Target, output: &mut Vec<u32>) {
    let [w, h] = *target.bounds().as_array();
    output.clear();
    output.reserve((w*h) as usize);

    for y in 0..h as usize {
        let y = (h - 1) as usize - y;

        for x in 0..(w / 8) as usize {
            let rgba = target[(x, y)];
            let argb = argb_u8x8_pack(rgba);
            output.extend(argb.to_array());
        }

        let rem = (w % 8) as usize;
        if rem > 0 {
            let rgba = target[((w/8) as usize, y)];
            let argb = argb_u8x8_pack(rgba);
            output.extend(&argb.to_array()[0..rem]);
        }
    }
}

fn target_to_argb_at(target: &Target, w: usize, h: usize, output: &mut Vec<u32>, start: usize, stride: usize) {
    for y in 0..h {
        let offset = start + y*stride;

        let y = (h - 1) - y;

        for x in 0..(w / 8) {
            let rgba = target[(x, y)];
            let argb = argb_u8x8_pack(rgba);
            for dx in 0..8 {
                output[offset + 8*x + dx] = argb.as_array()[dx];
            }
        }

        let rem = w % 8;
        if rem > 0 {
            let x = w/8;
            let rgba = target[(x, y)];
            let argb = argb_u8x8_pack(rgba);
            for dx in 0..rem {
                output[offset + 8*x + dx] = argb.as_array()[dx];
            }
        }
    }
}


fn parse_xml(xml: &str) -> Svg<'static> {
    let xml = roxmltree::Document::parse(xml).unwrap();

    let root = xml.root();
    let svg = root.children().next().unwrap();

    let mut result = Svg { commands: vec![] };

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

                let path = pb.build();

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

                        result.commands.push(SvgCommand::Fill(path.clone(), color));
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

                        result.commands.push(SvgCommand::Stroke(path.clone(), color, width));
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

    result
}


fn rasterize_fill(tile: Rect, path: &Path) -> Option<(U32x2, Mask<'static>)> {
    let mask_aabb = path.aabb.clamp_to(tile).round_inclusive_fast();

    let mask_w = mask_aabb.width()  as u32;
    let mask_h = mask_aabb.height() as u32;
    if mask_w == 0 || mask_h == 0 {
        return None;
    }

    let p0 = mask_aabb.min;

    let mut r = Rasterizer::new(mask_w, mask_h);
    r.fill_path(path, p0);

    let offset = (p0 - tile.min).0.cast();
    Some((offset, r.accumulate()))
}

fn rasterize_stroke(tile: Rect, path: &Path, width: F32) -> Option<(U32x2, Mask<'static>)> {
    let path_aabb = path.aabb.grow(F32x2::splat(width/2.0));
    let mask_aabb = path_aabb.clamp_to(tile).round_inclusive_fast();

    let mask_w = mask_aabb.width()  as u32;
    let mask_h = mask_aabb.height() as u32;
    if mask_w == 0 || mask_h == 0 {
        return None;
    }

    let p0 = mask_aabb.min;

    let mut r = Rasterizer::new(mask_w, mask_h);
    r.stroke_path(path, width/2.0, width/2.0, p0);

    let offset = (p0 - tile.min).0.cast();
    Some((offset, r.accumulate()))
}


fn render_svg(svg: &Svg, w: u32, h: u32) -> Vec<u32> {
    let mut output = vec![];
    _render_svg(svg, w, h, &mut output);
    output
}

#[inline(never)]
fn _render_svg(svg: &Svg, w: u32, h: u32, output: &mut Vec<u32>) {
    output.clear();
    output.reserve((w*h) as usize);
    unsafe { output.set_len((w*h) as usize) };

    let tile_size = 165;
    let mut tile_target = Target::new(tile_size, tile_size);

    let tiles_x = (w + tile_size - 1) / tile_size;
    let tiles_y = (h + tile_size - 1) / tile_size;
    let tile_count = (tiles_x * tiles_y) as usize;

    let mut paths = 0;
    let mut pixels = 0;

    let t0 = std::time::Instant::now();

    let path_count = svg.commands.len();
    let visible_count = (path_count + 63)/64;
    let mut visible = vec![0; visible_count*tile_count];

    for (command_index, command) in svg.commands.iter().enumerate() {

        fn fill_visible(visible: &mut Vec<u64>, visible_count: usize, path_index: usize, path_aabb: Rect,
            tile_size: u32, tiles_x: u32, tiles_y: u32
        ) {
            let path_bit = 1 << (path_index as u64 % 64);

            let tiles_end = F32x2::new(tiles_x as F32, tiles_y as F32);

            let tile_size = F32x2::splat(tile_size as F32);
            let rect = rect(path_aabb.min / tile_size, path_aabb.max / tile_size);
            let rect = rect.round_inclusive_fast();
            let begin: U32x2 = rect.min.clamp(F32x2::zero(), tiles_end).0.cast();
            let end:   U32x2 = rect.max.clamp(F32x2::zero(), tiles_end).0.cast();

            for y in begin[1]..end[1] {
                for x in begin[0]..end[0] {
                    let base = (y*tiles_x + x) as usize * visible_count;
                    visible[base + path_index/64] |= path_bit;
                }
            }
        }

        match command {
            SvgCommand::Fill (path, _color) => {
                fill_visible(&mut visible, visible_count, command_index, path.aabb, tile_size, tiles_x, tiles_y);
            },

            SvgCommand::Stroke (path, _color, width) => {
                let aabb = path.aabb.grow(F32x2::splat(width/2.0));
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
                            if let Some((offset, mask)) = rasterize_fill(tile, path) {
                                paths += 1;
                                pixels += (mask.width() * mask.height()) as usize;
                                fill_mask(&mut tile_target, offset, &mask, *color);
                            }
                        },

                        SvgCommand::Stroke (path, color, width) => {
                            if let Some((offset, mask)) = rasterize_stroke(tile, path, *width) {
                                paths += 1;
                                pixels += (mask.width() * mask.height()) as usize;
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


    let dt = t0.elapsed();
    let dt_path = dt.as_secs_f32() * 1000.0 * 1000.0 / paths as f32;
    let dt_pix = dt.as_secs_f32() * 1000.0 * 1000.0 * 1000.0 / pixels as f32;
    let size = (tile_target.stride() * tile_target.height() as usize) * core::mem::size_of::<[F32x8; 4]>();

    if 1==1 {
        println!("{}x{}, {} kiB", w, h, size / 1024);
        println!("{} paths in {:.2?}", paths, dt);
        println!("{:.2?}us per path", dt_path);
        println!("{:.2?}ns per pixel", dt_pix);
        println!("{} pixels per path", pixels / paths);
    }
}

