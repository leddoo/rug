//use rug::*;
// @temp
use rug::geometry::*;
use rug::image::*;
use rug::renderer::*;

fn main() {
    // spall::init()

    spall::touch();

    spall::trace_scope!("main");
    {
        spall::trace_scope!("my secret sauce"; "{:?}", 33 + 36);
    }

    let mut work_dt = 0;
    let t0 = std::time::Instant::now();
    let mut j = 3;
    for _ in 0..10_000_000 {
        spall::trace_scope!("foo");
        let work_t0 = spall::rdtsc();
        for _ in 0..100 {
            if j % 2 == 0 {
                j = j/2;
            }
            else {
                j = 3*j + 1;
            }
        }
        let work_t1 = spall::rdtsc();
        work_dt += work_t1 - work_t0;
    }
    let dt = t0.elapsed();
    println!("{:?} - {:?}", dt, dt/10_000_000);
    println!("{}", work_dt as f64 * 41.0 / 10_000_000.0);

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

