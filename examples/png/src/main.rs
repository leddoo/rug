//use rug::*;
// @temp
use rug::geometry::*;
use rug::image::*;
use rug::renderer::*;
use rug::cmd::*;
use rug::color::*;

fn draw_svg(name: &str, svg: &str, w: u32, h: u32, s: f32, flip: bool) {
    println!("drawing {:?}", name);
    //spall::trace_scope!("draw_svg", name);

    let cmd_buf = vg_inputs::parse_svg(svg);

    let mut target = Image::new([w, h]);


    let params = RenderParams {
        clear: 0xffffffff,
        tfx: if flip {
            Transform::translate([0.0, h as f32].into()) *
            Transform::scale([s, -s].into())
        }
        else {
            Transform::scale([s, s].into())
        },
    };

    let t0 = std::time::Instant::now();
    let mut iters = 0;
    while t0.elapsed() < std::time::Duration::from_secs(5) {
        render(&cmd_buf, &params, &mut target.img_mut());
        iters += 1;
        break;
    }
    let dt = t0.elapsed() / iters;
    println!("{name:?}: {dt:?}, {iters} iters, {w}x{h}, {:?} per pixel", dt/(w*h));

    let path = format!("target/{name}.png");
    ::image::save_buffer(path, target.as_bytes(), target.width(), target.height(), ::image::ColorType::Rgba8).unwrap();
}


fn main() {
    //spall::init("target/trace.spall").unwrap();
    //spall::touch();

    //draw_svg("firefox", &std::fs::read_to_string("target/firefox.svg").unwrap(), 770, 800, 10.0, false);

    if 1==1 {
        draw_svg("car", vg_inputs::svg::CAR, 900, 600, 1.0, true);
        draw_svg("gallardo", vg_inputs::svg::GALLARDO, 1901, 1018, 1.0, false);
        draw_svg("gradient_tri", vg_inputs::svg::GRADIENT_TRI, 1024, 1024, 1.0, false);
        draw_svg("intertwingly", vg_inputs::svg::INTERTWINGLY, 1024, 1024, 1.0, false);
        draw_svg("paris", vg_inputs::svg::PARIS, 1024, 1024, 1.0, true);
        draw_svg("radial_gradient_1", vg_inputs::svg::RADIAL_GRADIENT_1, 1024, 1024, 1.0, false);
        draw_svg("scimitar", vg_inputs::svg::SCIMITAR, 466, 265, 1.0, false);
        draw_svg("tiger", vg_inputs::svg::TIGER, 495, 510, 1.0, true);
        draw_svg("tommek_car", vg_inputs::svg::TOMMEK_CAR, 1052, 744, 1.0, false);
    }

    // gradients.
    if 0==1 {
        let cmd_buf = CmdBuf::new(|cb| {
            let stops = cb.build_gradient_stops(|sb| {
                sb.push(GradientStop {
                    offset: 0.0,
                    color: argb_pack_u8s(255, 0, 0, 255),
                });
                sb.push(GradientStop {
                    offset: 0.333,
                    color: argb_pack_u8s(255, 255, 0, 255),
                });
                sb.push(GradientStop {
                    offset: 0.667,
                    color: argb_pack_u8s(255, 0, 0, 255),
                });
                sb.push(GradientStop {
                    offset: 1.0,
                    color: argb_pack_u8s(0, 255, 0, 255),
                });
            });

            /*
            let alpha = 35.0/180.0 * core::f32::consts::PI;
            let rotation = Transform {
                columns: [
                    [ alpha.cos(), alpha.sin()].into(),
                    [-alpha.sin(), alpha.cos()].into(),
                    [0.0, 0.0].into(),
                ],
            };
            */

            let gradient = cb.push_radial_gradient(RadialGradient {
                cp: [200.0, 200.0].into(), cr: 4.0,
                fp: [200.0, 200.0].into(), fr: 0.0,
                spread: SpreadMethod::Pad,
                units:  GradientUnits::Absolute,
                /*
                tfx:    Transform::translate([200.0, 200.0].into()) *
                        rotation *
                        Transform::scale([1.0, 0.5].into()) *
                        Transform::translate([-200.0, -200.0].into()),
                */
                tfx: Transform::ID(),
                stops,
            });

            let path = cb.build_path(|pb| {
                pb.move_to([100.0, 300.0]);
                pb.line_to([300.0, 300.0]);
                pb.line_to([300.0, 100.0]);
                pb.line_to([100.0, 100.0]);
                pb.close_path();
            });

            cb.push(Cmd::FillPathRadialGradient { path, gradient, opacity: 1.0 });
        });


        let w = 400;
        let h = 400;
        let s = 1.0;

        let mut target = Image::new([w, h]);

        /*
        let alpha = 15.0/180.0 * core::f32::consts::PI;
        let rotation = Transform {
            columns: [
                [ alpha.cos(), alpha.sin()].into(),
                [-alpha.sin(), alpha.cos()].into(),
                [0.0, 0.0].into(),
            ],
        };
        let rotation =
            Transform::translate([200.0, 200.0].into()) *
            rotation *
            Transform::translate([-200.0, -200.0].into());
        */

        let params = RenderParams {
            clear: 0xffffffff,
            tfx: Transform::scale1(s),// * rotation,
        };

        let iters = 1;
        let t0 = std::time::Instant::now();
        for _ in 0..iters {
            render(&cmd_buf, &params, &mut target.img_mut());
        }
        println!("{:?}", t0.elapsed()/iters);

        ::image::save_buffer("target/radial-gradient.png", target.as_bytes(), target.width(), target.height(), ::image::ColorType::Rgba8).unwrap();
    }

    // car.
    if 0==1 {
        let car = vg_inputs::parse_svg(vg_inputs::svg::CAR);
        println!("{}", car.num_cmds());

        let w = 900;
        let h = 600;
        let s = 1.0;

        let mut target = Image::new([w, h]);

        let params = RenderParams {
            clear: 0xffffffff,
            tfx: Transform::translate([0.0, h as f32].into()) *
                 Transform::scale([s, -s].into()),
        };

        let iters = 1000;
        let t0 = std::time::Instant::now();
        for _ in 0..iters {
            render(&car, &params, &mut target.img_mut());
        }
        println!("{:?}", t0.elapsed()/iters);
    }

    // paris.
    if 0==1 {
        let paris = vg_inputs::parse_svg(vg_inputs::svg::PARIS);
        println!("{}", paris.num_cmds());

        let w = 1024;
        let h = 1024;
        let s = 1.0;

        let mut target = Image::new([w, h]);

        let params = RenderParams {
            clear: 0xffffffff,
            tfx: Transform::translate([0.0, h as f32].into()) *
                 Transform::scale([s, -s].into()),
        };

        let iters = 100;
        let t0 = std::time::Instant::now();
        for _ in 0..iters {
            render(&paris, &params, &mut target.img_mut());
        }
        println!("{:?}", t0.elapsed()/iters);
    }

    // tiger.
    if 0==1 {
        let tiger = vg_inputs::parse_svg(vg_inputs::svg::TIGER);

        let w = 512;
        let h = 512;
        let s = 1.0;

        let mut target = Image::new([w, h]);

        let params = RenderParams {
            clear: 0xffffffff,
            tfx: Transform::scale([s, -s].into()) *
                 Transform::translate([0.0, -510.0].into()),
        };

        let iters = 500;
        let t0 = std::time::Instant::now();
        for _ in 0..iters {
            render(&tiger, &params, &mut target.img_mut());
        }
        let dt = t0.elapsed() / iters;
        println!("{:?}, {:?} per path", dt, dt / tiger.num_cmds() as u32);
    }
}

