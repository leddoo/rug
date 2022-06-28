//#![no_std]
#![feature(allocator_api)]
#![feature(portable_simd)]

pub use basic::*;
pub use basic::simd::*;

pub mod alloc;
pub mod float;
pub mod geometry;
pub mod path;
pub mod stroke;
pub mod image;
pub mod rasterizer;
pub mod pipeline;
pub mod renderer;

pub use alloc::*;
pub use float::*;
pub use geometry::*;
pub use path::*;
pub use stroke::*;
pub use image::*;
pub use rasterizer::*;
pub use pipeline::*;
pub use renderer::*;
