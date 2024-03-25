use sti::simd::*;

use crate::geometry::*;
use crate::path::*;
use crate::rasterizer::ZERO_TOLERANCE_SQ;


// @temp
pub fn stroke(path: Path, width: f32) -> PathBuf {
    //spall::trace_scope!("rug::stroke");
    return Stroker::stroke(path, width/2.0, width/2.0);
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
    fn stroke(path: Path, left: f32, right: f32) -> PathBuf {
        let mut s = Stroker {
            left,
            right: -right,
            tol_sq:  0.05 * 0.05,
            max_rec: 16,
            pb:  PathBuilder::new(),
            pbl: RawPathBuilder::new(),
            pbr: RawPathBuilder::new(),
        };

        let mut iter = path.iter();
        while let Some(e) = iter.next() {
            match e {
                IterEvent::Begin(_, _) => {}

                IterEvent::Line (line)  => s.line(line),
                IterEvent::Quad (quad)  => s.quad(quad),
                IterEvent::Cubic(cubic) => s.cubic(cubic),

                IterEvent::End(_, closed) => {
                    s.build_stroke(closed);
                }
            }
        }
        debug_assert!(s.pbl.verbs .is_empty());
        debug_assert!(s.pbl.points.is_empty());
        debug_assert!(s.pbr.verbs .is_empty());
        debug_assert!(s.pbr.points.is_empty());

        return s.pb.build();
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


    fn build_stroke(&mut self, closed: bool) {
        debug_assert!(self.pb.in_path() == false);

        let mut prev: Option<F32x2> = None;

        let points = &*self.pbl.points;
        let mut p = 0;
        for verb in &self.pbl.verbs {
            match verb {
                Verb::Line => {
                    let p0 = points[p + 0];
                    let p1 = points[p + 1];
                    p += 2;

                    if let Some(prev) = prev {
                        if p0 != prev {
                            // bevel join.
                            self.pb.line_to(p0);
                        }
                    }
                    else {
                        self.pb.move_to(p0);
                    }

                    self.pb.line_to(p1);

                    prev = Some(p1);
                }

                Verb::Quad => {
                    let p0 = points[p + 0];
                    let p1 = points[p + 1];
                    let p2 = points[p + 2];
                    p += 3;

                    if let Some(prev) = prev {
                        if p0 != prev {
                            // bevel join.
                            self.pb.line_to(p0);
                        }
                    }
                    else {
                        self.pb.move_to(p0);
                    }

                    self.pb.quad_to(p1, p2);

                    prev = Some(p2);
                }

                Verb::Cubic => {
                    // cubics are flattened to quads.
                    unreachable!()
                }

                Verb::BeginOpen | Verb::BeginClosed |
                Verb::EndOpen   | Verb::EndClosed   => unreachable!()
            }
        }
        debug_assert_eq!(p, points.len());


        if closed {
            self.pb.close_path();
            prev = None;
        }


        let points = &*self.pbr.points;
        let mut p = points.len();
        for verb in self.pbr.verbs.iter().rev() {
            match verb {
                Verb::Line => {
                    p -= 2;
                    let p0 = points[p + 1];
                    let p1 = points[p + 0];

                    if let Some(prev) = prev {
                        if p0 != prev {
                            // bevel join.
                            self.pb.line_to(p0);
                        }
                    }
                    else {
                        self.pb.move_to(p0);
                    }

                    self.pb.line_to(p1);

                    prev = Some(p1);
                }

                Verb::Quad => {
                    p -= 3;
                    let p0 = points[p + 2];
                    let p1 = points[p + 1];
                    let p2 = points[p + 0];

                    if let Some(prev) = prev {
                        if p0 != prev {
                            // bevel join.
                            self.pb.line_to(p0);
                        }
                    }
                    else {
                        self.pb.move_to(p0);
                    }

                    self.pb.quad_to(p1, p2);

                    prev = Some(p2);
                }

                Verb::Cubic => {
                    // cubics are flattened to quads.
                    unreachable!()
                }

                Verb::BeginOpen | Verb::BeginClosed |
                Verb::EndOpen   | Verb::EndClosed   => unreachable!()
            }
        }
        debug_assert_eq!(p, 0);

        if self.pbl.verbs.len() > 0 || self.pbr.verbs.len() > 0 {
            self.pb.close_path();
        }

        self.pbl.clear();
        self.pbr.clear();
    }


    #[allow(dead_code)]
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


