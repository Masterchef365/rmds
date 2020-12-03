#![allow(unused)]
use anyhow::Result;
use genmap::{GenMap, Handle};
use std::path::Path;

pub struct Engine;

#[derive(Copy, Clone)]
pub struct Buffer(pub(crate) Handle);

#[derive(Copy, Clone)]
pub struct Shader(pub(crate) Handle);

impl Engine {
    pub fn new() -> Result<Self> {
        todo!()
    }

    pub fn buffer(&mut self, len: usize) -> Result<Buffer> {
        todo!()
    }

    pub fn write(&mut self, buffer: Buffer, data: &[u8]) -> Result<()> {
        todo!()
    }

    pub fn read(&mut self, buffer: Buffer, data: &mut [u8]) -> Result<()> {
        todo!()
    }

    pub fn spirv(&mut self, spv: &[u8]) -> Result<Shader> {
        todo!()
    }

    #[cfg(feature = "shaderc")]
    pub fn glsl(&mut self, glsl: &str) -> Result<Shader> {
        todo!()
    }

    pub fn run(&mut self, shader: Shader, buffer: Buffer, x: u32, y: u32, z: u32) -> Result<()> {
        todo!()
    }
}
