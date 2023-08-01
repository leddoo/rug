//use rug::*;
// @temp
use rug::geometry::*;
use rug::image::*;
use rug::renderer::*;

fn main() {
    spall::init("target/trace.spall").unwrap();
    spall::touch();

    // paris.
    if 1==1 {
        let paris = vg_inputs::parse_svg(vg_inputs::PARIS_SVG);

        let mut target = Image::new([1024, 1024]);

        let params = RenderParams {
            clear: 0xffffffff,
            tfx: Transform::scale([1.0, -1.0].into()) *
                 Transform::translate([0.0, -1024.0].into()),
        };
        render(paris.cmds(), &params, &mut target.img_mut());

        ::image::save_buffer("target/paris.png", target.as_bytes(), target.width(), target.height(), ::image::ColorType::Rgba8).unwrap();
    }

    if 0==1 {
        let mut pb = rug::path::PathBuilder::new();
        pb.move_to([1.0, 1.0].into());
        pb.line_to([2.0, 1.0].into());
        pb.quad_to([2.0, 2.0].into(), [3.0, 2.0].into());
        pb.move_to([4.0, 1.0].into());
        pb.cubic_to([6.0, 1.0].into(), [4.0, 3.0].into(), [6.0, 3.0].into());
        pb.move_to([2.0, 3.0].into());
        pb.line_to([4.0, 3.0].into());
        pb.line_to([4.0, 4.0].into());
        pb.line_to([2.0, 4.0].into());
        pb.close_path();
        let path = pb.build();
        let path = path.path();

        path.ggb();
        println!("\n\n");

        let s = rug::stroke::stroke(path, 0.1);
        s.path().ggb();
        println!("\n\n");

        let s2 = rug::stroke::stroke(s.path(), 0.2);
        s2.path().ggb();
        println!("\n\n");
    }

    // tiger.
    if 1==1 {
        let tiger = vg_inputs::parse_svg(vg_inputs::TIGER_SVG);

        let mut target = Image::new([1024, 1024]);

        let params = RenderParams {
            clear: 0xffffffff,
            tfx: Transform::scale([2.0, -2.0].into()) *
                 Transform::translate([0.0, -510.0].into()),
        };
        render(tiger.cmds(), &params, &mut target.img_mut());

        ::image::save_buffer("target/tiger.png", target.as_bytes(), target.width(), target.height(), ::image::ColorType::Rgba8).unwrap();

        if 0==1 {
            let iters = 500;
            let t0 = std::time::Instant::now();
            for _ in 0..iters {
                render(tiger.cmds(), &params, &mut target.img_mut());
            }
            let dt = t0.elapsed() / iters;
            println!("{:?}, {:?} per path", dt, dt / tiger.cmds().len() as u32);
        }
    }
}

