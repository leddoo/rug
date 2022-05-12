#![feature(portable_simd)]

mod win32;

use rug::*;

fn main() {
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
        let size = window.size();
        if size != old_size {
            old_size = size;

            let (w, h) = size;
            let buffer = render_svg(&svg, w, h);
            window.fill_pixels(&buffer, 0, 0, w, h);
        }
        else {
            std::thread::sleep(std::time::Duration::from_millis(7));
        }
    }
}


struct Svg<'a> {
    paths: Vec<(Path<'a>, F32x4)>,
}



use ttf_parser as ttf;

#[allow(dead_code)]
#[inline(never)]
fn render(face: &ttf_parser::Face, w: u32, h: u32) -> Vec<u32> {

    let t0 = std::time::Instant::now();

    const FONT_SIZE: f32 = 12.0;

    let units_per_em = face.units_per_em();
    let scale = FONT_SIZE / units_per_em as f32;

    let cell_size = face.height() as f32 * FONT_SIZE / units_per_em as f32;
    let cell_size = cell_size.ceil() as u32;

    let columns = w / cell_size;
    

    let mut image = Target::new(w, h);

    let mut row = 0;
    let mut column = 0;
    for id in 0..face.number_of_glyphs() {
        let mut r = Rasterizer::new(cell_size, cell_size);

        glyph_to_path(
            &face,
            ttf::GlyphId(id),
            scale,
            &mut r,
        );

        let offset = U32x2::from([column, row]) * U32x2::splat(cell_size);
        fill_mask(&mut image, offset, &r.accumulate(), F32x4::splat(1.0));

        column += 1;
        if column == columns {
            column = 0;
            row += 1;
        }

        if row * cell_size > h {
            break;
        }
    }

    let buffer = target_to_argb(image);

    let count = (row * columns) as u32;

    if 1 == 1 {
    let dt = t0.elapsed();
    println!("done.");
    println!("  rendered {} glyphs in {:.2?}", count, dt);
    println!("  cell_size: {}", cell_size);
    println!("  cells on screen: {}", w as f32 * h as f32 / (cell_size * cell_size) as f32);
    println!("  window size: {}", w*h);
    println!("  time per cell: {:.2?}", dt / count);
    println!("  time per pixel: {:.2?}", dt / count / (cell_size * cell_size));
    }

    buffer
}

#[allow(dead_code)]
fn glyph_to_path(
    face: &ttf::Face,
    glyph_id: ttf::GlyphId,
    scale: f32,
    rasterizer: &mut Rasterizer,
) {
    let mut builder = Builder { r: rasterizer, p0: v2f(0.0, 0.0), s: scale };

    let _bbox = match face.outline_glyph(glyph_id, &mut builder) {
        Some(v) => v,
        None => return,
    };

    struct Builder<'r, 'a> {
        s: f32,
        r:  &'r mut Rasterizer<'a>,
        p0: V2f,
    }

    impl ttf::OutlineBuilder for Builder<'_, '_> {
        fn move_to(&mut self, x: f32, y: f32) {
            self.p0 = self.s*v2f(x, y);
        }

        fn line_to(&mut self, x: f32, y: f32) {
            let p1 = self.s*v2f(x, y);
            self.r.add_segment_p(self.p0, p1);
            self.p0 = p1;
        }

        fn quad_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32) {
            let p1 = self.s*v2f(x1, y1);
            let p2 = self.s*v2f(x2, y2);
            self.r.add_quadratic_p(self.p0, p1, p2);
            self.p0 = p2;
        }

        fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x3: f32, y3: f32) {
            let p1 = self.s*v2f(x1, y1);
            let p2 = self.s*v2f(x2, y2);
            let p3 = self.s*v2f(x3, y3);
            self.r.add_cubic_p(self.p0, p1, p2, p3);
            self.p0 = p3;
        }

        fn close(&mut self) {
        }
    }
}

fn target_to_argb(target: Target) -> Vec<u32> {
    let w = target.width();
    let h = target.height();

    let mut buffer = vec![];
    buffer.reserve((w*h) as usize);

    for y in 0..h as usize {
        let y = (h - 1) as usize - y;

        // TODO: gamma correct.

        for x in 0..(w / 8) as usize {
            let rgba = target[(x, y)];
            let argb = argb_u8x8_pack(rgba);
            buffer.extend(argb.to_array());
        }

        let rem = (w % 8) as usize;
        if rem > 0 {
            let rgba = target[((w/8) as usize, y)];
            let argb = argb_u8x8_pack(rgba);
            buffer.extend(&argb.to_array()[0..rem]);
        }
    }

    buffer
}


fn parse_xml(xml: &str) -> Svg<'static> {
    let xml = roxmltree::Document::parse(xml).unwrap();

    let root = xml.root();
    let svg = root.children().next().unwrap();

    let mut result = Svg { paths: vec![] };

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

                let fill = node.attribute("fill")?;
                let color = svgtypes::Color::from_str(fill).ok()?;

                let alpha =
                    if node.has_attribute("fill-opacity") {
                        svgtypes::Number::from_str(node.attribute("fill-opacity").unwrap()).unwrap().0 as f32
                    }
                    else {
                        1.0
                    };

                use svgtypes::PathSegment::*;

                fn to_v2f(x: f64, y: f64) -> V2f {
                    v2f(x as f32, y as f32)
                }

                let mut pb = PathBuilder::new();
                for curve in svgtypes::PathParser::from(node.attribute("d").unwrap()) {
                    match curve.unwrap() {
                        MoveTo { abs, x, y } => {
                            assert!(abs);
                            pb.move_to(to_v2f(x, y));
                        },

                        LineTo { abs, x: x1, y: y1 } => {
                            assert!(abs);
                            pb.segment_to(to_v2f(x1, y1));
                        },

                        Quadratic { abs, x1, y1, x: x2, y: y2 } => {
                            assert!(abs);
                            pb.quadratic_to(to_v2f(x1, y1), to_v2f(x2, y2));
                        },

                        CurveTo { abs, x1, y1, x2, y2, x: x3, y: y3 } => {
                            assert!(abs);
                            pb.cubic_to(to_v2f(x1, y1), to_v2f(x2, y2), to_v2f(x3, y3));
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

                let a = alpha * (color.alpha as f32 / 255.0);
                let color = F32x4::from([
                    color.red   as f32 / 255.0,
                    color.green as f32 / 255.0,
                    color.blue  as f32 / 255.0,
                    a,
                ]);

                result.paths.push((path, color));
            },

            _ => {
                println!("unknown tag: {}", tag_name.name());
            },
        }

        Some(())
    }

    result
}


#[inline(never)]
fn render_svg(svg: &Svg, w: u32, h: u32) -> Vec<u32> {
    let mut image = Target::new(w, h);
    image.clear(F32x4::from([0.0, 0.0, 0.0, 1.0]));

    let mut paths = 0;
    let mut pixels = 0;


    let t0 = std::time::Instant::now();

    let image_aabb = rect(v2f(0.0, 0.0), v2f(image.width() as f32, image.height() as f32));

    for (path, color) in &svg.paths {
        let half_size = v2f(w as f32 / 2.0, h as f32 / 2.0);
        let visible = path.aabb().grow(half_size).contains(half_size);
        if !visible {
            continue;
        }

        let mask_aabb = path.aabb().clamp_to(image_aabb).round_inclusive_fast();

        let mask_w = mask_aabb.width()  as u32;
        let mask_h = mask_aabb.height() as u32;
        if mask_w == 0 || mask_h == 0 {
            continue;
        }

        let p0 = mask_aabb.min;

        let mut r = Rasterizer::new(mask_w, mask_h);
        path.iter(|curve| {
            use Curve::*;
            match curve {
                Segment(segment)     => { r.add_segment(segment + -p0); },
                Quadratic(quadratic) => { r.add_quadratic(quadratic + -p0); },
                Cubic(cubic)         => { r.add_cubic(cubic + -p0); },
            }
        });

        paths += 1;
        pixels += (mask_w*mask_h) as usize;

        fill_mask(&mut image, U32x2::from([p0.x as u32, p0.y as u32]), &r.accumulate(), *color);
    }

    let dt = t0.elapsed();
    let dt_path = dt.as_secs_f32() * 1000.0 * 1000.0 / paths as f32;
    let dt_pix = dt.as_secs_f32() * 1000.0 * 1000.0 * 1000.0 / pixels as f32;
    let size = (image.stride() * h as usize) * core::mem::size_of::<[F32x8; 4]>();

    if 1==1 {
        println!("{} kiB", size / 1024);
        println!("{} paths in {:.2?}", paths, dt);
        println!("{:.2?}us per path", dt_path);
        println!("{:.2?}ns per pixel", dt_pix);
        println!("{} pixels per path", pixels / paths);
    }


    target_to_argb(image)
}


#[allow(dead_code)]
fn render_debug(w: u32, h: u32) -> Vec<u32> {
    let mut image = Target::new(w, h);

    image.clear(F32x4::splat(1.0));

    let mut r = Rasterizer::new(w, h);


    use svgtypes::PathSegment::*;

    let mut p0 = v2f(0.0, 0.0);

    let mut initial = None;

    fn to_v2f(x: f64, y: f64) -> V2f {
        //v2f(x as f32 - 116.0, y as f32 - 139.0)
        v2f(x as f32, y as f32)
    }

    //let d = "M 142.632,157.137 C 142.632,157.137 141.098,157.137 137.262,155.219 135.344,155.219 124.603,151.766 119.232,142.176 119.232,142.176 131.124,151.383 142.632,157.137 ";
    //let d = "M 85.405,13.0275 L 86.5375,14.4675 88.0075,16.3375 90.3175,19.2925 91.7237,21.06 92.1338,20.7325 93.4663,19.6975 94.2088,19.1163 94.37,17.9975 93.6087,16.9625 89.56,11.485 88.77,10.42 86.4212,12.2363 85.405,13.0275 85.405,13.0275 Z ";
    let d = "M 86.69,19.4137 L 88.0375,21.1625 88.3887,21.6213 89.375,22.8763 90.5175,21.9925 91.7237,21.06 90.3175,19.2925 89.78,19.6975 89.3262,19.1313 88.3937,19.8488 87.5287,18.735 86.69,19.4138 86.69,19.4137 Z M 88.3987,20.8737 L 88.6862,20.65 89.0287,21.1037 88.955,21.1675 88.8325,21.0063 88.6188,21.1625 88.3988,20.8737 88.3987,20.8737 Z ";

    for curve in svgtypes::PathParser::from(d) {
        match curve.unwrap() {
            MoveTo { abs, x, y } => {
                assert!(abs);
                p0 = to_v2f(x, y);

                if initial.is_none() {
                    initial = Some(p0);
                }
            },

            LineTo { abs, x: x1, y: y1 } => {
                assert!(abs);
                let p1 = to_v2f(x1, y1);

                r.add_segment_p(p0, p1);
                p0 = p1;
            },

            Quadratic { abs, x1, y1, x: x2, y: y2 } => {
                assert!(abs);
                let p1 = to_v2f(x1, y1);
                let p2 = to_v2f(x2, y2);

                r.add_quadratic_p(p0, p1, p2);
                p0 = p2;
            },

            CurveTo { abs, x1, y1, x2, y2, x: x3, y: y3 } => {
                assert!(abs);
                let p1 = to_v2f(x1, y1);
                let p2 = to_v2f(x2, y2);
                let p3 = to_v2f(x3, y3);

                r.add_cubic_p(p0, p1, p2, p3);
                p0 = p3;
            },

            ClosePath { abs } => {
                assert!(abs);
                let p1 = initial.unwrap_or(v2f(0.0, 0.0));
                initial = None;

                r.add_segment_p(p0, p1);
                p0 = p1;
            },

            _ => {
                unreachable!()
            },
        }
    }

    let color = F32x4::from([1.0, 0.0, 1.0, 1.0]);
    fill_mask(&mut image, U32x2::from([0, 0]), &r.accumulate(), color);

    target_to_argb(image)
}
