use sti::simd::*;

use crate::geometry::*;
use crate::path::*;
use crate::rasterizer::ZERO_TOLERANCE_SQ;


// @temp
pub fn stroke(path: Path, size: f32) {
    Stroker::stroke(path, size, size);
}


struct Stroker {
    left:   f32,
    right:  f32,
    tol_sq:  f32,
    max_rec: u32,

    pb:  PathBuilder,
    pbl: RawPathBuilder,
    pbr: RawPathBuilder,
}

impl Stroker {
    fn stroke(path: Path, left: f32, right: f32) {
        let mut s = Stroker {
            left,
            right: -right,
            tol_sq:  0.05 * 0.05,
            max_rec: 16,
            pb:  PathBuilder::new(),
            pbl: RawPathBuilder::new(),
            pbr: RawPathBuilder::new(),
        };

        // the issue with joins is,
        // we need to know about both curves.
        // for miter/round, that is.
        // so thinking, just push everything raw.
        // then, in post pass, join, if end points differ.

        // joins: ! do bevel if within like 2*tolerance.
        // cause otherwise we do some nasty stuff for
        // discontinuous cubic approximations.

        let mut iter = path.iter();
        while let Some(e) = iter.next() {
            match e {
                IterEvent::Begin(_, _) => {
                }

                IterEvent::Line (line)  => s.line(line),
                IterEvent::Quad (quad)  => s.quad(quad),
                IterEvent::Cubic(cubic) => s.cubic(cubic),

                IterEvent::End(_, _) => {
                    s.dbg();
                    s.pbl.clear();
                    s.pbr.clear();
                }
            }
        }
    }


    fn push_line(&mut self, line: Line, normal: F32x2) {
        let l = line.offset(normal, self.left);
        self.pbl.verbs.push(Verb::Line);
        self.pbl.points.push(l.p0);
        self.pbl.points.push(l.p1);

        let r = line.offset(normal, self.right);
        self.pbr.verbs.push(Verb::Line);
        self.pbr.points.push(r.p0);
        self.pbr.points.push(r.p1);
    }

    fn line(&mut self, line: Line) {
        if let Some(normal) = line.normal(ZERO_TOLERANCE_SQ) {
            self.push_line(line, normal);
        }
    }

    fn quad_ex(&mut self, quad: Quad, tol_sq: f32, max_rec: u32) {
        let Quad { p0, p1, p2 } = quad;

        if (p2 - p0).length_sq() <= ZERO_TOLERANCE_SQ {
            self.line(line(p0, p1));
            self.line(line(p1, p2));
            return;
        }

        match quad.normals(ZERO_TOLERANCE_SQ) {
            (Some(n0), Some(n1)) => {
                quad.offset(n0, n1, self.left, tol_sq, max_rec, &mut |q, _| {
                    self.pbl.verbs.push(Verb::Quad);
                    self.pbl.points.push(q.p0);
                    self.pbl.points.push(q.p1);
                    self.pbl.points.push(q.p2);
                });

                quad.offset(n0, n1, self.right, tol_sq, max_rec, &mut |q, _| {
                    self.pbr.verbs.push(Verb::Quad);
                    self.pbr.points.push(q.p0);
                    self.pbr.points.push(q.p1);
                    self.pbr.points.push(q.p2);
                });
            },

            (Some(n0), None) => {
                self.push_line(line(p0, p2), n0);
            },

            (None, Some(n1)) => {
                self.push_line(line(p0, p2), n1);
            },

            _ => {
                // should be unreachable.
                // implies p0 = p1 = p2, but we've checked p0 ≠ p2 above.
                #[cfg(debug_assertions)]
                unreachable!()
            }
        }
    }

    fn quad(&mut self, quad: Quad) {
        self.quad_ex(quad, self.tol_sq, self.max_rec);
    }

    fn cubic(&mut self, cubic: Cubic) {
        let tol = self.tol_sq / 4.0;
        let rec = self.max_rec / 2;
        cubic.reduce(tol, rec, &mut |q, rec_left| {
            self.quad_ex(q, tol, rec + rec_left);
        });
    }

    fn dbg(&self) {
        fn dbg_path(verbs: &[Verb], points: &[F32x2]) {
            let mut p = 0;
            for v in verbs {
                match *v {
                    Verb::Line => {
                        line(points[p], points[p+1]).ggb();
                        p += 2;
                    }

                    Verb::Quad => {
                        quad(points[p], points[p+1], points[p+2]).ggb();
                        p += 3;
                    }

                    Verb::Cubic => {
                        cubic(points[p], points[p+1], points[p+2], points[p+3]).ggb();
                        p += 4;
                    }

                    _ => unreachable!()
                }
            }
        }

        dbg_path(&self.pbl.verbs, &self.pbl.points);
        println!();
        dbg_path(&self.pbr.verbs, &self.pbr.points);
        println!("\n");
    }
}


