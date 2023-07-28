//use rug::*;
// @temp
use rug::geometry::*;
use rug::image::*;
use rug::renderer::*;

fn main() {
    spall::trace_scope!("main");
    {
        spall::trace_scope!("my secret sauce"; "{:?}", 33 + 36);
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
            let iters = 3000;
            let t0 = std::time::Instant::now();
            for _ in 0..iters {
                render(cmds.cmds(), &params, &mut target.img_mut());
            }
            let dt = t0.elapsed() / iters;
            println!("{:?}, {:?} per path", dt, dt / cmds.cmds().len() as u32);
        }
    }
}

