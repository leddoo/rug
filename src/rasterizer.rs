use crate::geometry::*;


pub struct Rasterizer {
    width:  usize,
    height: usize,
    deltas: Vec<f32>,
}


impl Rasterizer {
    pub fn new(width: usize, height: usize) -> Rasterizer {
        Rasterizer {
            width, height,
            deltas: vec![0.0; width*height],
        }
    }

    pub fn width(&self)  -> usize { self.width }
    pub fn height(&self) -> usize { self.height }


    //fn add_delta_unchecked() {}

    pub fn add_segment(&mut self, seg: Segment) {

        /*
            todo:
                - clamp to height.
                - don't divide by zero.
                - add_delta & clamp to width.

            - padding: +1 in y; +2 in x.
                - floor of clamped y will always yield valid row.
                - same for x, but x needs one more row because of add_delta().
        */

        let w = self.width as f32;
        let h = self.height as f32;

        // TODO: clamp.
        let ((x0, y0), (x1, y1)) = (seg.p0.into(), seg.p1.into());

        let dx = x1 - x0;
        let dy = y1 - y0;

        let x_step = 1f32.copysign(dx);
        let y_step = 1f32.copysign(dy);

        let x_nudge = if x0 <= x1 { 0f32 } else { 1f32 };
        let y_nudge = if y0 <= y1 { 0f32 } else { 1f32 };

        let x_dt = 1f32/dx;
        let y_dt = 1f32/dy;

        let x_i0 = x0.floor();
        let y_i0 = y0.floor();
        let x_i1 = x1.floor();
        let y_i1 = y1.floor();

        let steps = ((x_i1 - x_i0).abs() + (y_i1 + y_i0).abs()) as usize;


        let mut x_i = x_i0 as i32;
        let mut y_i = y_i0 as i32;

        let mut x_prev = x0;
        let mut y_prev = y0;
        let mut x_next = x_i0 + x_step + x_nudge;
        let mut y_next = y_i0 + y_step + y_nudge;
        let mut x_t_next = (x_next - x0) / dx;
        let mut y_t_next = (y_next - y0) / dy;

        for _ in 0..steps {
            //println!("Polygon(({}, {}), ({}, {}), ({}, {}), ({}, {})),",
                //x_i, y_i, x_i + 1, y_i, x_i + 1, y_i + 1, x_i, y_i + 1);

            let x;
            let y;
            if x_t_next < y_t_next {
                x = x_next;
                y = y0 + x_t_next*dy;

                x_i      += 1;
                x_next   += x_step;
                x_t_next += x_dt
            }
            else {
                x = x0 + y_t_next*dx;
                y = y_next;

                y_i      += 1;
                y_next   += y_step;
                y_t_next += y_dt
            }

            //println!("Segment(({}, {}), ({}, {})),", x_prev, y_prev, x, y);

            // add_delta(x_prev, x, y_prev, y);

            x_prev = x;
            y_prev = y;
        }

        // add_delta(x_prev, x1, y_prev, y1);

    }
}
