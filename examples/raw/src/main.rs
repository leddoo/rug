use rug::*;

fn main() {
    let w = 10;
    let h = 5;

    let mut i = image::Image::new([0, 0]);
    let mut r = rasterizer::Rasterizer::new(&mut i, [w, h]);

    r.add_segment_p([1.0, 1.0].into(), [9.0, 1.0].into());
    r.add_segment_p([9.0, 1.0].into(), [5.0, 4.0].into());
    r.add_segment_p([5.0, 4.0].into(), [1.0, 1.0].into());

    let mask = r.accumulate();

    let ramp = " .:-=+*#%@";

    for y in 0..h as usize {
        for x in 0..w as usize {
            let a = mask[(x, y)];
            let i = (a * (ramp.len() - 1) as f32) as usize;
            print!("{}", ramp.as_bytes()[i] as char);
        }
        println!();
    }
}

