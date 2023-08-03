use sti::alloc::Alloc;
use sti::growing_arena::GrowingArena;
use sti::vec::Vec;
use sti::keyed::{KSlice, KVec};
use sti::simd::*;

use crate::geometry::Transform;
use crate::path::{Path, PathBuilder};


#[derive(Clone, Copy, Debug)]
pub enum Cmd<'a> {
    // @todo: color abstraction.
    FillPathSolid   { path: Path<'a>, color: u32 },
    StrokePathSolid { path: Path<'a>, color: u32, width: f32 },
    FillPathLinearGradient { path: Path<'a>, gradient: LinearGradientId, opacity: f32 },
    FillPathRadialGradient { path: Path<'a>, gradient: RadialGradientId, opacity: f32 },
}



#[derive(Clone, Copy, Debug)]
pub struct GradientStop {
    pub offset:  f32,
    pub color:   u32,
}


#[derive(Clone, Copy, Debug)]
pub enum SpreadMethod {
    Pad,
    Reflect,
    Repeat,
}

#[derive(Clone, Copy, Debug)]
pub enum GradientUnits {
    Absolute,
    Relative,
}


sti::define_key!(u32, pub LinearGradientId);

#[derive(Clone, Debug)]
pub struct LinearGradient<'a> {
    pub p0: F32x2,
    pub p1: F32x2,
    pub spread: SpreadMethod,
    pub units:  GradientUnits,
    pub tfx:    Transform,
    pub stops:  &'a [GradientStop],
}


sti::define_key!(u32, pub RadialGradientId);

#[derive(Clone, Debug)]
pub struct RadialGradient<'a> {
    pub cp: F32x2,
    pub cr: f32,
    pub fp: F32x2,
    pub fr: f32,
    pub spread: SpreadMethod,
    pub units:  GradientUnits,
    pub tfx:    Transform,
    pub stops:  &'a [GradientStop],
}


pub struct CmdBuf {
    #[allow(dead_code)]
    arena: Box<GrowingArena>,

    cmds: &'static [Cmd<'static>],

    linear_gradients: &'static KSlice<LinearGradientId, LinearGradient<'static>>,
    radial_gradients: &'static KSlice<RadialGradientId, RadialGradient<'static>>,
}

impl CmdBuf {
    pub fn new<F: FnOnce(&mut CmdBufBuilder)>(f: F) -> Self {
        let arena = Box::new(GrowingArena::new());

        let (cmds, linear_gradients, radial_gradients) = {
            let mut builder = CmdBufBuilder {
                arena: arena.as_ref(),
                path_builder: PathBuilder::new(),
                gradient_stops_builder: Vec::new(),
                linear_gradients: KVec::new_in(arena.as_ref()),
                radial_gradients: KVec::new_in(arena.as_ref()),
                cmds: Vec::new_in(arena.as_ref()),
            };

            f(&mut builder);

            let cmds = unsafe { core::mem::transmute(Vec::leak(builder.cmds)) };

            // @temp
            let linear_gradients = unsafe {
                core::mem::transmute(
                    <KSlice::<LinearGradientId, LinearGradient>>::new_unck(
                        Vec::leak(builder.linear_gradients.into_inner())))
            };

            // @temp
            let radial_gradients = unsafe {
                core::mem::transmute(
                    <KSlice::<RadialGradientId, RadialGradient>>::new_unck(
                        Vec::leak(builder.radial_gradients.into_inner())))
            };

            (cmds, linear_gradients, radial_gradients)
        };

        CmdBuf { arena, cmds, linear_gradients, radial_gradients }
    }

    #[inline(always)]
    pub fn num_cmds(&self) -> usize {
        self.cmds.len()
    }

    #[inline(always)]
    pub fn cmd(&self, i: usize) -> &Cmd {
        &self.cmds[i]
    }

    #[inline(always)]
    pub fn linear_gradient(&self, id: LinearGradientId) -> &LinearGradient {
        &self.linear_gradients[id]
    }

    #[inline(always)]
    pub fn radial_gradient(&self, id: RadialGradientId) -> &RadialGradient {
        &self.radial_gradients[id]
    }
}



pub struct CmdBufBuilder<'a> {
    arena: &'a GrowingArena,

    path_builder: PathBuilder,

    gradient_stops_builder: Vec<GradientStop>,
    linear_gradients: KVec<LinearGradientId, LinearGradient<'a>, &'a GrowingArena>,
    radial_gradients: KVec<RadialGradientId, RadialGradient<'a>, &'a GrowingArena>,

    cmds: Vec<Cmd<'a>, &'a GrowingArena>,
}

impl<'a> CmdBufBuilder<'a> {
    #[inline(always)]
    pub fn alloc(&self) -> &'a impl Alloc { self.arena }

    #[inline(always)]
    pub fn build_path<F: FnOnce(&mut PathBuilder)>(&mut self, f: F) -> Path<'a> {
        self.path_builder.clear();
        f(&mut self.path_builder);
        self.path_builder.build_in(self.arena).leak()
    }

    #[inline(always)]
    pub fn build_gradient_stops<F: FnOnce(&mut Vec<GradientStop>)>(&mut self, f: F) -> &'a [GradientStop] {
        self.gradient_stops_builder.clear();
        f(&mut self.gradient_stops_builder);
        Vec::leak(self.gradient_stops_builder.clone_in(self.arena))
    }

    #[inline(always)]
    pub fn push_linear_gradient(&mut self, gradient: LinearGradient<'a>) -> LinearGradientId {
        self.linear_gradients.push(gradient)
    }

    #[inline(always)]
    pub fn push_radial_gradient(&mut self, gradient: RadialGradient<'a>) -> RadialGradientId {
        self.radial_gradients.push(gradient)
    }

    #[inline(always)]
    pub fn push(&mut self, cmd: Cmd<'a>) {
        self.cmds.push(cmd);
    }
}


