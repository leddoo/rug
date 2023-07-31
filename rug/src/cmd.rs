use sti::alloc::Alloc;
use sti::growing_arena::GrowingArena;
use sti::vec::Vec;

use crate::path::{Path, PathBuilder};


#[derive(Clone, Copy, Debug)]
pub enum Cmd<'a> {
    // @todo: color abstraction.
    FillPathSolid   { path: Path<'a>, color: u32 },
    StrokePathSolid { path: Path<'a>, color: u32, width: f32 },
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


pub struct CmdBuf {
    #[allow(dead_code)]
    arena: Box<GrowingArena>,

    cmds: &'static [Cmd<'static>],
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

        CmdBuf { arena, cmds }
    }

    #[inline(always)]
    pub fn cmds(&self) -> &[Cmd] { &self.cmds }
}


