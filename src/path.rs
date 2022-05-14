extern crate alloc;
use alloc::{
    alloc::{Allocator, Global},
    boxed::Box,
    vec::Vec,
};

use crate::wide::*;
use crate::geometry::*;


#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Verb {
    Move,
    Segment,
    Quadratic,
    Cubic,
    Close,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Curve {
    Segment   (Segment),
    Quadratic (Quadratic),
    Cubic     (Cubic),
}

#[derive(Clone)]
pub struct Path<'a> {
    pub verbs:  Box<[Verb], &'a dyn Allocator>,
    pub points: Box<[F32x2],  &'a dyn Allocator>,
    pub aabb:   Rect,
}

impl<'a> Path<'a> {
    #[inline(always)]
    pub fn iter<F: FnMut(Curve)>(&self, mut f: F) {
        let mut iter = Iter::new(self);

        for verb in self.verbs.iter() {
            match *verb {
                Verb::Move      => { iter.mov() },
                Verb::Segment   => { f(Curve::Segment(iter.segment())) },
                Verb::Quadratic => { f(Curve::Quadratic(iter.quadratic())) },
                Verb::Cubic     => { f(Curve::Cubic(iter.cubic())) },
                Verb::Close     => { f(Curve::Segment(iter.close())) },
            }
        }
    }
}


pub struct Iter<'p, 'a> {
    path: &'p Path<'a>,
    initial: F32x2,
    p0: F32x2,
    point: usize,
}

impl<'p, 'a> Iter<'p, 'a> {
    #[inline(always)]
    pub fn new(path: &'p Path<'a>) -> Iter<'p, 'a> {
        Iter {
            path,
            initial: F32x2::zero(),
            p0:      F32x2::zero(),
            point:   0
        }
    }

    #[inline(always)]
    pub fn mov(&mut self) {
        self.p0       = self.path.points[self.point];
        self.initial  = self.p0;
        self.point   += 1;
    }

    #[inline(always)]
    pub fn segment(&mut self) -> Segment {
        let p0 = self.p0;
        let p1 = self.path.points[self.point];
        self.point += 1;
        self.p0 = p1;
        segment(p0, p1)
    }

    #[inline(always)]
    pub fn quadratic(&mut self) -> Quadratic {
        let p0 = self.p0;
        let p1 = self.path.points[self.point + 0];
        let p2 = self.path.points[self.point + 1];
        self.point += 2;
        self.p0 = p2;
        quadratic(p0, p1, p2)
    }

    #[inline(always)]
    pub fn cubic(&mut self) -> Cubic {
        let p0 = self.p0;
        let p1 = self.path.points[self.point + 0];
        let p2 = self.path.points[self.point + 1];
        let p3 = self.path.points[self.point + 2];
        self.point += 3;
        self.p0 = p3;
        cubic(p0, p1, p2, p3)
    }

    #[inline(always)]
    pub fn close(&mut self) -> Segment {
        let p0 = self.p0;
        let p1 = self.initial;
        self.p0 = p1;
        segment(p0, p1)
    }
}


pub struct PathBuilder<'a> {
    verbs:  Vec<Verb,  &'a dyn Allocator>,
    points: Vec<F32x2, &'a dyn Allocator>,
    aabb:   Rect,
    p0:     F32x2,
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
            p0:     F32x2::zero(),
        }
    }


    pub fn build(self) -> Path<'a> {
        Path {
            verbs:  self.verbs.into_boxed_slice(),
            points: self.points.into_boxed_slice(),
            aabb:   self.aabb,
        }
    }


    pub fn move_to(&mut self, p0: F32x2) {
        self.verbs.push(Verb::Move);
        self.points.push(p0);
        self.aabb.include(p0);
        self.p0 = p0;
    }

    pub fn segment_to(&mut self, p1: F32x2) {
        self.verbs.push(Verb::Segment);
        self.points.push(p1);
        self.aabb.include(p1);
    }

    pub fn quadratic_to(&mut self, p1: F32x2, p2: F32x2) {
        self.verbs.push(Verb::Quadratic);
        self.points.push(p1);
        self.points.push(p2);
        self.aabb.include(p1);
        self.aabb.include(p2);
    }

    pub fn cubic_to(&mut self, p1: F32x2, p2: F32x2, p3: F32x2) {
        self.verbs.push(Verb::Cubic);
        self.points.push(p1);
        self.points.push(p2);
        self.points.push(p3);
        self.aabb.include(p1);
        self.aabb.include(p2);
        self.aabb.include(p3);
    }

    pub fn close(&mut self) {
        self.verbs.push(Verb::Close);
    }
}
