extern crate alloc;
use alloc::{
    alloc::{Allocator, Global},
    boxed::Box,
    vec::Vec,
};

use basic::{*, simd::*};
use crate::geometry::*;


#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Verb {
    Begin,
    BeginClosed,
    Segment,
    Quadratic,
    Cubic,
    End,
}

// (invariant) verbs regex:
//  `(Begin (Segment | Quadratic | Cubic)* (End | EndClosed))*`
#[derive(Clone)]
pub struct Path<'a> {
    pub verbs:  Box<[Verb],  &'a dyn Allocator>,
    pub points: Box<[F32x2], &'a dyn Allocator>,
    pub aabb:   Rect,
}

impl<'a> Path<'a> {
    #[inline(always)]
    pub fn iter(&self) -> Iter {
        Iter::new(self)
    }
}


pub enum IterEvent {
    Begin     (F32x2, Bool), // first-point, closed
    Segment   (Segment),
    Quadratic (Quadratic),
    Cubic     (Cubic),
    End       (F32x2), // last-point
}

pub struct Iter<'p, 'a> {
    path: &'p Path<'a>,
    pub(crate) verb:  usize,
    pub(crate) point: usize,
    pub(crate) p0:    F32x2,
}

impl<'p, 'a> Iter<'p, 'a> {
    #[inline(always)]
    pub fn new(path: &'p Path<'a>) -> Iter<'p, 'a> {
        Iter {
            path,
            verb:  0,
            point: 0,
            p0:    F32x2::ZERO,
        }
    }

    pub fn has_next(&self) -> bool {
        self.verb < self.path.verbs.len()
    }

    pub fn next(&mut self) -> Option<IterEvent> {
        let path = self.path;

        if !self.has_next() {
            debug_assert!(self.verb  == path.verbs.len());
            debug_assert!(self.point == path.points.len());
            return None;
        }

        let verb = path.verbs[self.verb];
        let result = match verb {
            Verb::Begin | Verb::BeginClosed => {
                let p0 = path.points[self.point];
                self.point += 1;
                self.p0 = p0;
                IterEvent::Begin(p0, verb == Verb::BeginClosed)
            },

            Verb::Segment => {
                let p0 = self.p0;
                let p1 = path.points[self.point];
                self.point += 1;
                self.p0 = p1;
                IterEvent::Segment(segment(p0, p1))
            },

            Verb::Quadratic => {
                let p0 = self.p0;
                let p1 = path.points[self.point + 0];
                let p2 = path.points[self.point + 1];
                self.point += 2;
                self.p0 = p2;
                IterEvent::Quadratic(quadratic(p0, p1, p2))
            },

            Verb::Cubic => {
                let p0 = self.p0;
                let p1 = path.points[self.point + 0];
                let p2 = path.points[self.point + 1];
                let p3 = path.points[self.point + 2];
                self.point += 3;
                self.p0 = p3;
                IterEvent::Cubic(cubic(p0, p1, p2, p3))
            },

            Verb::End => IterEvent::End(self.p0),
        };
        self.verb += 1;

        Some(result)
    }
}

impl<'p, 'a> Iterator for Iter<'p, 'a> {
    type Item = IterEvent;

    #[inline(always)]
    fn next(&mut self) -> Option<IterEvent> {
        self.next()
    }
}


pub struct PathBuilder<'a> {
    verbs:  Vec<Verb,  &'a dyn Allocator>,
    points: Vec<F32x2, &'a dyn Allocator>,
    aabb:   Rect,
    in_path:     Bool,
    begin_point: F32x2,
    begin_verb:  usize,
}

impl<'a> PathBuilder<'a> {
    pub fn new() -> PathBuilder<'a> {
        PathBuilder::new_in(&Global)
    }

    pub fn new_in(allocator: &'a dyn Allocator) -> PathBuilder<'a> {
        PathBuilder {
            verbs:  Vec::new_in(allocator),
            points: Vec::new_in(allocator),
            aabb:   rect(F32x2::splat(f32::MAX), F32x2::splat(f32::MIN)),
            in_path:     false,
            begin_point: F32x2::ZERO,
            begin_verb:  usize::MAX,
        }
    }


    pub fn build(mut self) -> Path<'a> {
        if self.in_path {
            self.verbs.push(Verb::End);
        }
        Path {
            verbs:  self.verbs.into_boxed_slice(),
            points: self.points.into_boxed_slice(),
            aabb:   self.aabb,
        }
    }

    pub fn move_to(&mut self, p0: F32x2) {
        if self.in_path {
            self.verbs.push(Verb::End);
        }
        self.verbs.push(Verb::Begin);
        self.points.push(p0);
        self.aabb.include(p0);
        self.in_path     = true;
        self.begin_point = p0;
        self.begin_verb  = self.verbs.len() - 1;
    }

    pub fn segment_to(&mut self, p1: F32x2) {
        assert!(self.in_path);
        self.verbs.push(Verb::Segment);
        self.points.push(p1);
        self.aabb.include(p1);
    }

    pub fn quadratic_to(&mut self, p1: F32x2, p2: F32x2) {
        assert!(self.in_path);
        self.verbs.push(Verb::Quadratic);
        self.points.push(p1);
        self.points.push(p2);
        self.aabb.include(p1);
        self.aabb.include(p2);
    }

    pub fn cubic_to(&mut self, p1: F32x2, p2: F32x2, p3: F32x2) {
        assert!(self.in_path);
        self.verbs.push(Verb::Cubic);
        self.points.push(p1);
        self.points.push(p2);
        self.points.push(p3);
        self.aabb.include(p1);
        self.aabb.include(p2);
        self.aabb.include(p3);
    }

    pub fn close(&mut self) {
        assert!(self.in_path);
        if *self.points.last().unwrap() != self.begin_point {
            self.segment_to(self.begin_point);
        }
        self.verbs[self.begin_verb] = Verb::BeginClosed;
        self.verbs.push(Verb::End);
        self.in_path    = false;
        self.begin_verb = usize::MAX;
    }
}


pub struct SoaPath<'a> {
    pub lines:  Box<[Segment],   &'a dyn Allocator>,
    pub quads:  Box<[Quadratic], &'a dyn Allocator>,
    pub cubics: Box<[Cubic],     &'a dyn Allocator>,
    pub aabb:   Rect,
}
