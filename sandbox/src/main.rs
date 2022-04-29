use minifb::{Key, Window, WindowOptions};

const WIDTH: usize = 16;
const HEIGHT: usize = 9;

fn main() {
    use rug::*;

    let mut r = Rasterizer::new(WIDTH, HEIGHT);
    r.add_segment(segment(v2f(5.5, 0.5), v2f(24.0, 11.0)));

    if 1==1 { return }


    let mut buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];

    let mut window = Window::new(
        "window",
        800,
        600,
        WindowOptions::default(),
    ).unwrap();

    window.limit_update_rate(Some(std::time::Duration::from_micros(16667)));

    while window.is_open() && !window.is_key_down(Key::Escape) {
        for (i, x) in buffer.iter_mut().enumerate() {
            *x = i as u32 * 7;
        }

        window.update_with_buffer(&buffer, WIDTH, HEIGHT).unwrap();
    }
}
