extern crate alloc;
use alloc::{
    alloc::{Allocator, Global},
    boxed::Box,
    vec::Vec,
};

use crate::geometry::*;


#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Verb {
    Move,
    Segment,
    Quadratic,
    Cubic,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Curve {
    Segment   (Segment),
    Quadratic (Quadratic),
    Cubic     (Cubic),
}

pub struct Path<'a> {
    verbs:  Box<[Verb], &'a dyn Allocator>,
    points: Box<[V2f],  &'a dyn Allocator>,
    aabb:   Rect,
}

impl<'a> Path<'a> {
    pub fn aabb(&self) -> Rect { self.aabb }

    pub fn iter<F: FnMut(Curve)>(&self, mut f: F) {
        let mut p0 = v2f(0.0, 0.0);

        let mut point = 0;

        for verb in self.verbs.iter() {
            match *verb {
                Verb::Move => {
                    p0 = self.points[point];
                    point += 1;
                },

                Verb::Segment => {
                    let p1 = self.points[point];
                    point += 1;

                    f(Curve::Segment(segment(p0, p1)));
                    p0 = p1;
                },

                Verb::Quadratic => {
                    let p1 = self.points[point + 0];
                    let p2 = self.points[point + 1];
                    point += 2;

                    f(Curve::Quadratic(quadratic(p0, p1, p2)));
                    p0 = p2;
                },

                Verb::Cubic => {
                    let p1 = self.points[point + 0];
                    let p2 = self.points[point + 1];
                    let p3 = self.points[point + 2];
                    point += 3;

                    f(Curve::Cubic(cubic(p0, p1, p2, p3)));
                    p0 = p3;
                },
            }
        }
    }
}


pub struct PathBuilder<'a> {
    verbs:  Vec<Verb, &'a dyn Allocator>,
    points: Vec<V2f,  &'a dyn Allocator>,
    aabb:   Rect,
    p0:     V2f,
}

impl<'a> PathBuilder<'a> {
    pub fn new() -> PathBuilder<'a> {
        PathBuilder::new_in(&Global)
    }

    pub fn new_in(allocator: &'a dyn Allocator) -> PathBuilder<'a> {
        PathBuilder {
            verbs:  Vec::new_in(allocator),
            points: Vec::new_in(allocator),
            aabb:   rect(v2f(f32::MAX, f32::MAX), v2f(f32::MIN, f32::MIN)),
            p0:     v2f(0.0, 0.0),
        }
    }


    pub fn build(self) -> Path<'a> {
        Path {
            verbs:  self.verbs.into_boxed_slice(),
            points: self.points.into_boxed_slice(),
            aabb:   self.aabb,
        }
    }


    pub fn move_to(&mut self, p0: V2f) {
        self.verbs.push(Verb::Move);
        self.points.push(p0);
        self.aabb.include(p0);
        self.p0 = p0;
    }

    pub fn segment_to(&mut self, p1: V2f) {
        self.verbs.push(Verb::Segment);
        self.points.push(p1);
        self.aabb.include(p1);
    }

    pub fn quadratic_to(&mut self, p1: V2f, p2: V2f) {
        self.verbs.push(Verb::Quadratic);
        self.points.push(p1);
        self.points.push(p2);
        self.aabb.include(p1);
        self.aabb.include(p2);
    }

    pub fn cubic_to(&mut self, p1: V2f, p2: V2f, p3: V2f) {
        self.verbs.push(Verb::Cubic);
        self.points.push(p1);
        self.points.push(p2);
        self.points.push(p3);
        self.aabb.include(p1);
        self.aabb.include(p2);
        self.aabb.include(p3);
    }

    pub fn close(&mut self) {
        self.segment_to(self.p0);
    }
}
