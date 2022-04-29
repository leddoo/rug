use rug::*;

fn main() {
    /*
    let mut r = Rasterizer::new(WIDTH, HEIGHT);
    r.add_segment_p(v2f(5.5, 0.5), v2f(24.0, 11.0));
    r.add_segment_p(v2f(24.0, 11.0), v2f(15.0, 2.0));
    r.add_segment_p(v2f(15.0, 2.0), v2f(5.5, 0.5));

    r.add_segment_p(v2f(-5.0, -5.0), v2f(5.0, 12.0));
    r.add_segment_p(v2f(5.0, 12.0), v2f(3.0, -5.0));

    r.add_segment_p(v2f(8.0, 5.5), v2f(8.0, 10.0));
    r.add_segment_p(v2f(9.5, 10.0), v2f(9.5, 5.5));


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
    */

    let bytes = include_bytes!(r"C:\Windows\Fonts\DejaVuSansMono.ttf");

    let face = ttf_parser::Face::from_slice(&bytes[..], 0).unwrap();


    let mut buffer_width  = 0;
    let mut buffer_height = 0;
    let mut buffer = vec![];


    let mut window = minifb::Window::new(
        "window", 800, 600,
        minifb::WindowOptions {
            resize: true,
            .. minifb::WindowOptions::default()
        },
    ).unwrap();

    window.limit_update_rate(Some(std::time::Duration::from_micros(16667)));

    while window.is_open() {
        let (w, h) = window.get_size();
        if w != buffer_width || h != buffer_height {
            buffer_width  = w;
            buffer_height = h;
            buffer = render(&face, w, h);
        }

        window.update_with_buffer(&buffer, buffer_width, buffer_height).unwrap();
    }
}


use ttf_parser as ttf;

fn render(face: &ttf_parser::Face, w: usize, h: usize) -> Vec<u32> {

    let t0 = std::time::Instant::now();

    const FONT_SIZE: f32 = 12.0;

    let units_per_em = face.units_per_em();
    let scale = FONT_SIZE / units_per_em as f32;

    let cell_size = face.height() as f32 * FONT_SIZE / units_per_em as f32;

    let columns = w / cell_size as usize;

    let mut r = Rasterizer::new(w, h);

    let mut row = 0;
    let mut column = 0;
    for id in 0..face.number_of_glyphs() {
        let x = column as f32 * cell_size;
        let y = row as f32 * cell_size;

        glyph_to_path(
            x,
            y,
            &face,
            ttf::GlyphId(id),
            cell_size,
            scale,
            &mut r,
        );

        column += 1;
        if column == columns {
            column = 0;
            row += 1;
        }

        if row as f32 * cell_size > h as f32 {
            break;
        }
    }

    let mut buffer: Vec<u32> = vec![0; w*h];

    for y in 0..r.height {
        let mut a = 0.0;
        for x in 0..r.width {
            a += r.deltas[y*r.stride + x];
            let a = a.abs().min(1.0).powf(1.0/2.2);
            let c = (a * 255.0) as u32;
            buffer[(h - 1 - y)*w + x] = c << 16 | c << 8 | c;
        }
    }

    let count = (row * columns) as u32;

    let dt = t0.elapsed();
    println!("done.");
    println!("  rendered {} glyphs in {:.2?}", count, dt);
    println!("  cell_size: {}", cell_size);
    println!("  cells on screen: {}", w as f32 * h as f32 / cell_size / cell_size);
    println!("  window size: {}", w*h);
    println!("  time per cell: {:.2?}", dt / count);
    println!("  time per pixel: {:.2?}", dt / count / (cell_size * cell_size) as u32);

    buffer
}

fn glyph_to_path(
    x: f32,
    y: f32,
    face: &ttf::Face,
    glyph_id: ttf::GlyphId,
    _cell_size: f32,
    scale: f32,
    rasterizer: &mut Rasterizer,
) {
    let mut builder = Builder { r: rasterizer, p0: v2f(0.0, 0.0), d: v2f(x, y), s: scale };

    let _bbox = match face.outline_glyph(glyph_id, &mut builder) {
        Some(v) => v,
        None => return,
    };

    struct Builder<'r> {
        d: V2f,
        s: f32,
        r:  &'r mut Rasterizer,
        p0: V2f,
    }

    impl ttf::OutlineBuilder for Builder<'_> {
        fn move_to(&mut self, x: f32, y: f32) {
            self.p0 = self.s*v2f(x, y) + self.d;
        }

        fn line_to(&mut self, x: f32, y: f32) {
            let p1 = self.s*v2f(x, y) + self.d;
            self.r.add_segment_p(self.p0, p1);
            self.p0 = p1;
        }

        fn quad_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32) {
            let p1 = self.s*v2f(x1, y1) + self.d;
            let p2 = self.s*v2f(x2, y2) + self.d;
            self.r.add_quadratic_p(self.p0, p1, p2);
            self.p0 = p2;
        }

        fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x3: f32, y3: f32) {
            let p1 = self.s*v2f(x1, y1) + self.d;
            let p2 = self.s*v2f(x2, y2) + self.d;
            let p3 = self.s*v2f(x3, y3) + self.d;
            self.r.add_cubic_p(self.p0, p1, p2, p3);
            self.p0 = p3;
        }

        fn close(&mut self) {
        }
    }
}
