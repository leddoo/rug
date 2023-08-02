//use rug::*;
// @temp
use rug::geometry::*;
use rug::image::*;
use rug::renderer::*;
use rug::cmd::*;
use rug::color::*;

fn main() {
    spall::init("target/trace.spall").unwrap();
    spall::touch();

    // gradients.
    if 1==1 {
        let cmd_buf = CmdBuf::new(|cb| {
            let stops = cb.build_gradient_stops(|sb| {
                sb.push(GradientStop {
                    offset: 0.0,
                    color: argb_pack_u8s(255, 0, 0, 255),
                });
                sb.push(GradientStop {
                    offset: 1.0,
                    color: argb_pack_u8s(0, 255, 0, 255),
                });
            });

            let gradient = cb.push_linear_gradient(LinearGradient {
                p0: [100.5, 100.5].into(),
                p1: [299.5, 299.5].into(),
                spread: SpreadMethod::Pad,
                units:  GradientUnits::Absolute,
                tfx:    Transform::ID,
                stops,
            });

            let path = cb.build_path(|pb| {
                pb.move_to([100.0, 300.0].into());
                pb.line_to([300.0, 300.0].into());
                pb.line_to([100.0, 100.0].into());
                pb.close_path();
            });

            cb.push(Cmd::FillPathLinearGradient { path, gradient, opacity: 1.0 });
        });


        let w = 400;
        let h = 400;
        let s = 1.0;

        let mut target = Image::new([w, h]);

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

        let params = RenderParams {
            clear: 0xffffffff,
            tfx: Transform::scale1(s) * rotation,
        };

        let iters = 1;
        let t0 = std::time::Instant::now();
        for _ in 0..iters {
            render(&cmd_buf, &params, &mut target.img_mut());
        }
        println!("{:?}", t0.elapsed()/iters);

        ::image::save_buffer("target/linear-gradient.png", target.as_bytes(), target.width(), target.height(), ::image::ColorType::Rgba8).unwrap();
    }

    // car.
    if 1==1 {
        let car = vg_inputs::parse_svg(vg_inputs::CAR_SVG);
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

        let iters = 1;
        let t0 = std::time::Instant::now();
        for _ in 0..iters {
        render(&car, &params, &mut target.img_mut());
        }
        println!("{:?}", t0.elapsed()/iters);

        ::image::save_buffer("target/car.png", target.as_bytes(), target.width(), target.height(), ::image::ColorType::Rgba8).unwrap();
    }

    // paris.
    if 0==1 {
        let paris = vg_inputs::parse_svg(vg_inputs::PARIS_SVG);
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

        ::image::save_buffer("target/paris.png", target.as_bytes(), target.width(), target.height(), ::image::ColorType::Rgba8).unwrap();
    }

    // tiger.
    if 0==1 {
        let tiger = vg_inputs::parse_svg(vg_inputs::TIGER_SVG);

        let w = 512;
        let h = 512;
        let s = 1.0;

        let mut target = Image::new([w, h]);

        let params = RenderParams {
            clear: 0xffffffff,
            tfx: Transform::scale([s, -s].into()) *
                 Transform::translate([0.0, -510.0].into()),
        };
        render(&tiger, &params, &mut target.img_mut());

        ::image::save_buffer("target/tiger.png", target.as_bytes(), target.width(), target.height(), ::image::ColorType::Rgba8).unwrap();

        if 0==1 {
            let iters = 500;
            let t0 = std::time::Instant::now();
            for _ in 0..iters {
                render(&tiger, &params, &mut target.img_mut());
            }
            let dt = t0.elapsed() / iters;
            println!("{:?}, {:?} per path", dt, dt / tiger.num_cmds() as u32);
        }
    }
}

