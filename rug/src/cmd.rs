use sti::alloc::Alloc;
use sti::growing_arena::GrowingArena;
use sti::vec::Vec;
use sti::keyed::KSlice;
use sti::simd::*;

use crate::geometry::Transform;
use crate::path::{Path, PathBuilder};


#[derive(Clone, Copy, Debug)]
pub enum Cmd<'a> {
    // @todo: color abstraction.
    FillPathSolid   { path: Path<'a>, color: u32 },
    StrokePathSolid { path: Path<'a>, color: u32, width: f32 },
    FillPathLinearGradient { path: Path<'a>, gradient: LinearGradientId },
}



#[derive(Clone, Copy, Debug)]
pub struct GradientStop {
    pub offset:  f32,
    pub color:   u32,
    pub opacity: f32,
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
    pub stops:  GradientStops<'a>
}

#[derive(Clone, Copy, Debug)]
pub enum GradientStops<'a> {
    Two ([GradientStop; 2]),
    N   (&'a [GradientStop]),
}



pub struct CmdBuf {
    #[allow(dead_code)]
    arena: Box<GrowingArena>,

    cmds: &'static [Cmd<'static>],

    linear_gradients: &'static KSlice<LinearGradientId, LinearGradient<'static>>,
}

impl CmdBuf {
    pub fn new<F: FnOnce(&mut CmdBufBuilder)>(f: F) -> Self {
        let arena = Box::new(GrowingArena::new());

        let cmds = {
            let mut builder = CmdBufBuilder {
                arena: arena.as_ref(),
                path_builder: PathBuilder::new(),
                cmds: Vec::new_in(arena.as_ref()),
            };

            f(&mut builder);

            unsafe { core::mem::transmute(Vec::leak(builder.cmds)) }
        };

        CmdBuf { arena, cmds, linear_gradients: KSlice::new_unck(&[]) }
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
}



pub struct CmdBufBuilder<'a> {
    arena: &'a GrowingArena,
    path_builder: PathBuilder,
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
    pub fn push(&mut self, cmd: Cmd<'a>) {
        self.cmds.push(cmd);
    }
}


