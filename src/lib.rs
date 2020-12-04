#![allow(unused)]
use anyhow::Result;
use erupt::vk1_0 as vk;
use genmap::{GenMap, Handle};
use gpu_alloc::{MemoryBlock, Request};
use gpu_alloc_erupt::EruptMemoryDevice;
use std::path::Path;
use vk_core::SharedCore;

struct StorageBuffer {
    buffer: vk::Buffer,
    allocation: Option<MemoryBlock<vk::DeviceMemory>>,
    length: usize,
}

pub struct Engine {
    buffers: GenMap<StorageBuffer>,
    shaders: GenMap<vk::Pipeline>,
    command_pool: vk::CommandPool,
    command_buffer: vk::CommandBuffer,
    descriptor_set_layout: vk::DescriptorSetLayout,
    descriptor_pool: vk::DescriptorPool,
    descriptor_set: vk::DescriptorSet,
    core: SharedCore,
}

#[derive(Copy, Clone)]
pub struct Buffer(pub(crate) Handle);

#[derive(Copy, Clone)]
pub struct Shader(pub(crate) Handle);

impl Engine {
    pub fn new(validation: bool) -> Result<Self> {
        // Core (contains instance, device, etc.)
        let (core, core_meta) = vk_core::Core::compute(validation, "RMDS")?;

        // Create command buffer
        // Command pool:
        let create_info = vk::CommandPoolCreateInfoBuilder::new()
            .queue_family_index(core_meta.queue_family_index)
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);
        let command_pool =
            unsafe { core.device.create_command_pool(&create_info, None, None) }.result()?;

        // Create command buffer
        let allocate_info = vk::CommandBufferAllocateInfoBuilder::new()
            .command_pool(command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);

        let command_buffer =
            unsafe { core.device.allocate_command_buffers(&allocate_info) }.result()?[0];

        // Create descriptor set layout
        let bindings = [vk::DescriptorSetLayoutBindingBuilder::new()
            .binding(0)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::COMPUTE)];

        let create_info = vk::DescriptorSetLayoutCreateInfoBuilder::new().bindings(&bindings);

        let descriptor_set_layout = unsafe {
            core.device
                .create_descriptor_set_layout(&create_info, None, None)
        }
        .result()?;

        // Create descriptor pool
        let pool_sizes = vec![vk::DescriptorPoolSizeBuilder::new()
            ._type(vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count(1)];

        // Create descriptor pool of appropriate size
        let create_info = vk::DescriptorPoolCreateInfoBuilder::new()
            .pool_sizes(&pool_sizes)
            .max_sets(1);
        let descriptor_pool =
            unsafe { core.device.create_descriptor_pool(&create_info, None, None) }.result()?;

        // Create descriptor set
        let descriptor_set_layouts = [descriptor_set_layout];
        let create_info = vk::DescriptorSetAllocateInfoBuilder::new()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&descriptor_set_layouts);
        let descriptor_set =
            unsafe { core.device.allocate_descriptor_sets(&create_info) }.result()?[0];

        Ok(Self {
            descriptor_set,
            descriptor_pool,
            descriptor_set_layout,
            buffers: GenMap::with_capacity(10),
            shaders: GenMap::with_capacity(10),
            core,
            command_buffer,
            command_pool,
        })
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

impl Drop for Engine {
    fn drop(&mut self) {
        unsafe {
            self.core.device.queue_wait_idle(self.core.queue);

            self.core
                .device
                .destroy_descriptor_pool(Some(self.descriptor_pool), None);
            self.core
                .device
                .destroy_command_pool(Some(self.command_pool), None);
            self.core
                .device
                .destroy_descriptor_set_layout(Some(self.descriptor_set_layout), None);
        }
    }
}
