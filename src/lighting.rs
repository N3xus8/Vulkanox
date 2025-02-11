use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use vulkano::{
    buffer::{Buffer, BufferCreateInfo, BufferUsage, Subbuffer},
    memory::allocator::{
        AllocationCreateInfo, FreeListAllocator, GenericMemoryAllocator, MemoryTypeFilter,
    },
};

use crate::error::Result;

#[repr(C)]
#[derive(Default, Debug, Copy, Clone, Pod, Zeroable)]
pub struct AmbientLight {
    pub color: [f32; 3],
    pub intensity: f32,
}

impl AmbientLight {
    pub fn setup_ambient_light_buffers(
        ambient_light: AmbientLight,
        memory_allocator: Arc<GenericMemoryAllocator<FreeListAllocator>>,
    ) -> Result<Subbuffer<AmbientLight>> {
        let ambient_light_buffer = Buffer::from_data(
            memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::UNIFORM_BUFFER | BufferUsage::TRANSFER_DST,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            ambient_light,
        )?;

        Ok(ambient_light_buffer)
    }
}

pub const WHITE_AMBIENT_LIGHT: AmbientLight = AmbientLight {
    color: [1.0, 1.0, 1.0],
    intensity: 1.0,
};

#[repr(C)]
#[derive(Default, Debug, Copy, Clone, Pod, Zeroable)]
pub struct DirectionalLight {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

impl DirectionalLight {
    pub fn setup_directional_light_buffers(
        directional_light: DirectionalLight,
        memory_allocator: Arc<GenericMemoryAllocator<FreeListAllocator>>,
    ) -> Result<Subbuffer<DirectionalLight>> {
        let directional_light_buffer = Buffer::from_data(
            memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::UNIFORM_BUFFER | BufferUsage::TRANSFER_DST,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            directional_light,
        )?;

        Ok(directional_light_buffer)
    }
}
