#![feature(portable_simd)]
#![feature(allocator_api)]

mod win32;

use sti::simd::*;
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

        let mut target =
            //RenderTarget::new(2560, 1440);
            //RenderTarget::new(2048, 2048);
            //RenderTarget::new(1920, 1080);
            RenderTarget::new(1024, 1024);
            //RenderTarget::new(512, 512);

        for _ in 0..iters {
            svg.execute(&mut target, tfx);
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
            let mut target = RenderTarget::new(w, h);

            let t0 = std::time::Instant::now();
            svg.execute(&mut target, tfx);
            println!("{:?}", t0.elapsed());

            window.fill_pixels(&target.data, 0, 0, w, h);
        }
    }
}


fn parse_xml(xml: &str) -> CommandBuffer<alloc::GlobalAlloc> {
    let xml = roxmltree::Document::parse(xml).unwrap();

    let root = xml.root();
    let svg = root.children().next().unwrap();

    let mut result = CommandBuffer::new();

    for child in svg.children() {
        render(&mut result, child);
    }

    return result;

    fn render(result: &mut CommandBuffer<GlobalAlloc>, node: roxmltree::Node) -> Option<()> {
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

                        let a = ((alpha * (color.alpha as f32 / 255.0)) * 255.0) as u8;
                        let color = argb_pack_u8s(color.red, color.green, color.blue, a);

                        result.add(Command::FillPathSolid {
                            path: path.clone(),
                            color,
                            rule: FillRule::NonZero,
                        });
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

                        let a = ((alpha * (color.alpha as f32 / 255.0)) * 255.0) as u8;
                        let color = argb_pack_u8s(color.red, color.green, color.blue, a);

                        result.add(Command::StrokePathSolid {
                            path: path.clone(),
                            color,
                            width,
                            cap: CapStyle::Butt,
                            join: JoinStyle::Bevel,
                        });
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
}

