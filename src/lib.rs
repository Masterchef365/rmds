#![allow(unused)]
use anyhow::{ensure, Context, Result};
use erupt::vk1_0 as vk;
use genmap::{GenMap, Handle};
use gpu_alloc::{MemoryBlock, Request, UsageFlags};
use gpu_alloc_erupt::EruptMemoryDevice;
use std::ffi::CString;
use std::path::Path;
use vk_core::SharedCore;
use bytemuck::Pod;

struct StorageBuffer {
    // TODO: Only one staging buffer! We're single threaded anyhow...
    staging_buffer: vk::Buffer,
    staging_allocation: MemoryBlock<vk::DeviceMemory>,
    buffer: vk::Buffer,
    allocation: MemoryBlock<vk::DeviceMemory>,
    size_bytes: usize,
}

pub struct Engine {
    buffers: GenMap<StorageBuffer>,
    shaders: GenMap<vk::Pipeline>,
    pipeline_layout: vk::PipelineLayout,
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

const SHADER_ENTRY: &str = "main";

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
        let bindings = [
            vk::DescriptorSetLayoutBindingBuilder::new()
                .binding(0)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::COMPUTE),
            vk::DescriptorSetLayoutBindingBuilder::new()
                .binding(1)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::COMPUTE),
        ];

        let create_info = vk::DescriptorSetLayoutCreateInfoBuilder::new().bindings(&bindings);

        let descriptor_set_layout = unsafe {
            core.device
                .create_descriptor_set_layout(&create_info, None, None)
        }
        .result()?;

        // Create descriptor pool
        let pool_sizes = vec![vk::DescriptorPoolSizeBuilder::new()
            ._type(vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count(2)];

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

        // Push cosntant ranges
        let push_constant_ranges = [vk::PushConstantRangeBuilder::new()
            .stage_flags(vk::ShaderStageFlags::COMPUTE)
            .offset(0)
            .size(128)];

        // Pipeline layout
        let create_info =
            vk::PipelineLayoutCreateInfoBuilder::new()
            .push_constant_ranges(&push_constant_ranges)
            .set_layouts(&descriptor_set_layouts);
        let pipeline_layout =
            unsafe { core.device.create_pipeline_layout(&create_info, None, None) }.result()?;

        Ok(Self {
            pipeline_layout,
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

    fn internal_create_buffer<T: Pod>(&mut self, mem_usage: UsageFlags, size_bytes: usize) -> Result<(MemoryBlock<vk::DeviceMemory>, vk::Buffer)> {
        // Create a staging buffer
        let create_info = vk::BufferCreateInfoBuilder::new()
            .usage(
                vk::BufferUsageFlags::STORAGE_BUFFER | 
                vk::BufferUsageFlags::TRANSFER_SRC | 
                vk::BufferUsageFlags::TRANSFER_DST
            )
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .size(size_bytes as _);

        let buffer =
            unsafe { self.core.device.create_buffer(&create_info, None, None) }.result()?;

        // Allocate memory for it
        use gpu_alloc::UsageFlags;
        let request = Request {
            size: size_bytes as _,
            align_mask: std::mem::align_of::<T>() as _,
            usage: mem_usage,
            memory_types: !0,
        };

        let allocation = unsafe {
            self.core
                .allocator()?
                .alloc(EruptMemoryDevice::wrap(&self.core.device), request)?
        };

        // Bind that memory
        unsafe {
            self.core
                .device
                .bind_buffer_memory(buffer, *allocation.memory(), allocation.offset())
                .result()?;
        }

        Ok((allocation, buffer))
    }

    pub fn buffer<T: Pod>(&mut self, length: usize) -> Result<Buffer> {
        ensure!(length > 0, "Buffer length must be > 0");
        let size_bytes = length * std::mem::size_of::<T>();

        let (staging_allocation, staging_buffer) = self.internal_create_buffer::<T>(
            UsageFlags::DOWNLOAD | UsageFlags::UPLOAD | UsageFlags::HOST_ACCESS,
            size_bytes,
        )?;

        let (allocation, buffer) = self.internal_create_buffer::<T>(
            UsageFlags::FAST_DEVICE_ACCESS,
            size_bytes,
        )?;


        let storage_buffer = StorageBuffer {
            allocation,
            buffer,
            staging_buffer,
            size_bytes,
            staging_allocation,
        };

        Ok(Buffer(self.buffers.insert(storage_buffer)))
    }

    fn transfer_buffer_internal(&mut self, src: vk::Buffer, dst: vk::Buffer, size_bytes: usize) -> Result<()> {
        let region = vk::BufferCopyBuilder::new()
            .src_offset(0)
            .dst_offset(0)
            .size(size_bytes as _);

        unsafe {
            self.core
                .device
                .reset_command_buffer(self.command_buffer, None)
                .result()?;
            let begin_info = vk::CommandBufferBeginInfoBuilder::new();
            self.core
                .device
                .begin_command_buffer(self.command_buffer, &begin_info)
                .result()?;
            self.core
                .device
                .cmd_copy_buffer(self.command_buffer, src, dst, &[region]);
            self.core
                .device
                .end_command_buffer(self.command_buffer)
                .result()?;
            let command_buffers = [self.command_buffer];
            let submit_info = vk::SubmitInfoBuilder::new().command_buffers(&command_buffers);
            self.core
                .device
                .queue_submit(self.core.queue, &[submit_info], None)
                .result()?;
            // TODO: Use a fence here!!
            self.core.device.queue_wait_idle(self.core.queue).result()?;
        }

        Ok(())
    }

    pub fn write<T: Pod>(&mut self, buffer: Buffer, data: &[T]) -> Result<()> {
        let buffer = self
            .buffers
            .get_mut(buffer.0)
            .context("Buffer was deleted")?;
        ensure!(std::mem::size_of_val(data) <= buffer.size_bytes, "Buffer size must best < gpu buffer!");
        unsafe {
            buffer
                .staging_allocation
                .write_bytes(EruptMemoryDevice::wrap(&self.core.device), 0, bytemuck::cast_slice(data))?;
        }
        let src = buffer.staging_buffer;
        let dst = buffer.buffer;
        let size_bytes = buffer.size_bytes;
        self.transfer_buffer_internal(src, dst, size_bytes)?;
        Ok(())
    }

    pub fn read<T: Pod>(&mut self, buffer: Buffer, data: &mut [T]) -> Result<()> {
        {
            let buffer = self
                .buffers
                .get_mut(buffer.0)
                .context("Buffer was deleted")?;
            ensure!(std::mem::size_of_val(data) <= buffer.size_bytes, "Buffer size must best < gpu buffer!");
            let src = buffer.buffer;
            let dst = buffer.staging_buffer;
            let size_bytes = buffer.size_bytes;
            self.transfer_buffer_internal(src, dst, size_bytes)?;
        }
        let buffer = self
            .buffers
            .get_mut(buffer.0)
            .context("Buffer was deleted")?;
        unsafe {
            buffer
                .staging_allocation
                .read_bytes(EruptMemoryDevice::wrap(&self.core.device), 0, bytemuck::cast_slice_mut(data))?;
        }
        Ok(())
    }

    pub fn spirv(&mut self, spv: &[u8]) -> Result<Shader> {
        // Create module
        let shader_decoded = erupt::utils::decode_spv(spv).context("Shader decode failed")?;
        let create_info = vk::ShaderModuleCreateInfoBuilder::new().code(&shader_decoded);
        let shader_module = unsafe {
            self.core
                .device
                .create_shader_module(&create_info, None, None)
        }
        .result()?;

        let entry_point = CString::new(SHADER_ENTRY)?;

        // Create stage
        let stage = vk::PipelineShaderStageCreateInfoBuilder::new()
            .stage(vk::ShaderStageFlagBits::COMPUTE)
            .module(shader_module)
            .name(&entry_point)
            .build();
        let create_info = vk::ComputePipelineCreateInfoBuilder::new()
            .stage(stage)
            .layout(self.pipeline_layout);

        // Create pipeline
        let pipeline = unsafe {
            self.core
                .device
                .create_compute_pipelines(None, &[create_info], None)
        }
        .result()?[0];

        // Clean up
        unsafe {
            self.core
                .device
                .destroy_shader_module(Some(shader_module), None);
        }

        Ok(Shader(self.shaders.insert(pipeline)))
    }

    #[cfg(feature = "shaderc")]
    pub fn glsl(&mut self, glsl: &str) -> Result<Shader> {
        // TODO: Memoize compiler?
        let mut compiler = shaderc::Compiler::new().context("Could not find shaderc")?;
        let binary_result = compiler.compile_into_spirv(
            glsl,
            shaderc::ShaderKind::Compute,
            "shader.glsl",
            SHADER_ENTRY,
            None,
        )?;

        self.spirv(binary_result.as_binary_u8())
    }

    pub fn run(
        &mut self,
        shader: Shader,
        read: Buffer,
        write: Buffer,
        x: u32,
        y: u32,
        z: u32,
        push_constant: &[u8],
    ) -> Result<()> {
        assert!(x > 0 && y > 0 && z > 0);
        let read = self
            .buffers
            .get(read.0)
            .context("Read buffer was deleted")?;
        let write = self
            .buffers
            .get(write.0)
            .context("Write buffer was deleted")?;
        let pipeline = *self.shaders.get(shader.0).context("Shader was deleted")?;

        unsafe {
            self.core.device.update_descriptor_sets(
                &[
                    vk::WriteDescriptorSetBuilder::new()
                        .dst_set(self.descriptor_set)
                        .dst_binding(0)
                        .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                        .buffer_info(&[vk::DescriptorBufferInfoBuilder::new()
                            .buffer(read.buffer)
                            .offset(0)
                            .range(vk::WHOLE_SIZE)]),
                    vk::WriteDescriptorSetBuilder::new()
                        .dst_set(self.descriptor_set)
                        .dst_binding(1)
                        .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                        .buffer_info(&[vk::DescriptorBufferInfoBuilder::new()
                            .buffer(write.buffer)
                            .offset(0)
                            .range(vk::WHOLE_SIZE)]),
                ],
                &[],
            );
            self.core
                .device
                .reset_command_buffer(self.command_buffer, None)
                .result()?;
            let begin_info = vk::CommandBufferBeginInfoBuilder::new();
            self.core
                .device
                .begin_command_buffer(self.command_buffer, &begin_info)
                .result()?;

            let push_constant = if push_constant.is_empty() {
                &[0; 4]
            } else {
                push_constant
            };
            self.core.device.cmd_push_constants(
                self.command_buffer,
                self.pipeline_layout,
                vk::ShaderStageFlags::COMPUTE,
                0,
                push_constant.len() as u32,
                push_constant.as_ptr() as _,
            );

            self.core.device.cmd_bind_descriptor_sets(
                self.command_buffer,
                vk::PipelineBindPoint::COMPUTE,
                self.pipeline_layout,
                0,
                &[self.descriptor_set],
                &[],
            );

            self.core.device.cmd_bind_pipeline(
                self.command_buffer,
                vk::PipelineBindPoint::COMPUTE,
                pipeline,
            );

            self.core.device.cmd_dispatch(self.command_buffer, x, y, z);

            self.core
                .device
                .end_command_buffer(self.command_buffer)
                .result()?;

            let command_buffers = [self.command_buffer];
            let submit_info = vk::SubmitInfoBuilder::new().command_buffers(&command_buffers);
            self.core
                .device
                .queue_submit(self.core.queue, &[submit_info], None)
                .result()?;
            self.core.device.queue_wait_idle(self.core.queue).result()?;
        }

        Ok(())
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        unsafe {
            self.core.device.queue_wait_idle(self.core.queue);

            for pipeline in self.shaders.iter() {
                let pipeline = self.shaders.get(pipeline).unwrap();
                self.core.device.destroy_pipeline(Some(*pipeline), None);
            }

            for buffer in self.buffers.iter().collect::<Vec<_>>() {
                let buffer = self.buffers.remove(buffer).unwrap();
                self.core.allocator().unwrap().dealloc(
                    EruptMemoryDevice::wrap(&self.core.device),
                    buffer.staging_allocation,
                );
                self.core.allocator().unwrap().dealloc(
                    EruptMemoryDevice::wrap(&self.core.device),
                    buffer.allocation,
                );
                self.core.device.destroy_buffer(Some(buffer.staging_buffer), None);
                self.core.device.destroy_buffer(Some(buffer.buffer), None);
            }

            self.core
                .device
                .destroy_descriptor_pool(Some(self.descriptor_pool), None);
            self.core
                .device
                .destroy_command_pool(Some(self.command_pool), None);
            self.core
                .device
                .destroy_pipeline_layout(Some(self.pipeline_layout), None);
            self.core
                .device
                .destroy_descriptor_set_layout(Some(self.descriptor_set_layout), None);
        }
    }
}
