const WIDTH: usize = 16;
const HEIGHT: usize = 9;

fn main() {
    use rug::*;

    let mut r = Rasterizer::new(WIDTH, HEIGHT);
    r.add_segment_ps(v2f(5.5, 0.5), v2f(24.0, 11.0));
    r.add_segment_ps(v2f(24.0, 11.0), v2f(15.0, 2.0));
    r.add_segment_ps(v2f(15.0, 2.0), v2f(5.5, 0.5));

    r.add_segment_ps(v2f(-5.0, -5.0), v2f(5.0, 12.0));
    r.add_segment_ps(v2f(5.0, 12.0), v2f(3.0, -5.0));

    r.add_segment_ps(v2f(8.0, 5.5), v2f(8.0, 10.0));
    r.add_segment_ps(v2f(9.5, 10.0), v2f(9.5, 5.5));


    let mut buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];

    for y in 0..r.height {
        let mut a = 0.0;
        for x in 0..r.width {
            a += r.deltas[y*r.stride + x];
            let a = a.abs().min(1.0);
            buffer[(HEIGHT - 1 - y)*WIDTH + x] = (a.powf(1.0/2.2) * 255.0) as u32;
            print!("{:.2} ", a);
        }
        println!();
    }
    println!();

    let mut window = minifb::Window::new(
        "window",
        800,
        450,
        minifb::WindowOptions::default(),
    ).unwrap();

    window.limit_update_rate(Some(std::time::Duration::from_micros(16667)));

    while window.is_open() {
        window.update_with_buffer(&buffer, WIDTH, HEIGHT).unwrap();
    }
}
