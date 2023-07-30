//use rug::*;
// @temp
use rug::geometry::*;
use rug::image::*;
use rug::renderer::*;

fn main() {
    spall::init("target/trace.spall").unwrap();
    spall::touch();

    {
        let mut pb = rug::path::PathBuilder::new();
        pb.move_to([1.0, 1.0].into());
        pb.line_to([2.0, 1.0].into());
        pb.quad_to([2.0, 2.0].into(), [3.0, 2.0].into());
        pb.move_to([3.0, 1.0].into());
        pb.cubic_to([5.0, 1.0].into(), [3.0, 3.0].into(), [5.0, 3.0].into());
        let path = pb.build();
        let path = path.path();

        for e in path.iter() {
            match e {
                rug::path::IterEvent::Begin(_, _) => (),
                rug::path::IterEvent::Line (l) => l.ggb(),
                rug::path::IterEvent::Quad (q) => q.ggb(),
                rug::path::IterEvent::Cubic(c) => c.ggb(),
                rug::path::IterEvent::End(_, _) => (),
            }
        }
        println!("\n\n");

        rug::stroke::stroke(path, 0.1);
    }

    // tiger.
    {
        let mut target = Image::new([512, 512]);

        let cmds = vg_inputs::tiger_static();

        let params = RenderParams {
            clear: 0xffffffff,
            tfx: Transform::scale([1.0, -1.0].into()) *
                 Transform::translate([0.0, -510.0].into()),
        };
        render(cmds.cmds(), &params, &mut target.img_mut());

        ::image::save_buffer("target/tiger.png", target.as_bytes(), target.width(), target.height(), ::image::ColorType::Rgba8).unwrap();

        if 0==1 {
            let iters = 500;
            let t0 = std::time::Instant::now();
            for _ in 0..iters {
                render(cmds.cmds(), &params, &mut target.img_mut());
            }
            let dt = t0.elapsed() / iters;
            println!("{:?}, {:?} per path", dt, dt / cmds.cmds().len() as u32);
        }
    }
}

