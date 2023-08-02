//use rug::*;
// @temp
use rug::geometry::*;
use rug::image::*;
use rug::renderer::*;

fn main() {
    spall::init("target/trace.spall").unwrap();
    spall::touch();

    // gradients.
    if 1==1 {
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
            tfx: Transform::translate([0.0, w as f32].into()) *
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

        let w = 4*1024;
        let h = 4*1024;
        let s = 8.0;

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

