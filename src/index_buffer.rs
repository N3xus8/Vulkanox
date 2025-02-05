use std::sync::Arc;

use crate::error::Result;
use vulkano::{
    buffer::{Buffer, BufferCreateInfo, BufferUsage, Subbuffer},
    memory::{
        allocator::{
            AllocationCreateInfo, FreeListAllocator, GenericMemoryAllocator, MemoryTypeFilter,
        },
        MemoryPropertyFlags,
    },
    DeviceSize,
};

pub fn setup_index_buffers(
    indices: Vec<u16>,
    memory_allocator: Arc<GenericMemoryAllocator<FreeListAllocator>>,
) -> Result<(Option<Subbuffer<[u32]>>, Option<Subbuffer<[u32]>>)> {
    let indices_length = indices.len();
    if indices_length > 0 {
        let indices: Vec<u32> = indices.iter().map(|id| *id as u32).collect();

        // Create an Staging index buffer : subbuffer<[u32]>

        let index_staging_buffer = Buffer::from_iter(
            memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::TRANSFER_SRC,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_HOST,
                ..Default::default()
            },
            indices,
        )?;
        // --->

        let index_buffer = Buffer::new_slice(
            memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::INDEX_BUFFER | BufferUsage::TRANSFER_DST,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter {
                    required_flags: MemoryPropertyFlags::DEVICE_LOCAL, // Make sure this buffer is on the Device=GPU
                    ..Default::default()
                },
                ..Default::default()
            },
            indices_length as DeviceSize,
        )?;

         Ok((Some(index_staging_buffer), Some(index_buffer)))
    } else {
         Ok((None, None))
    }
}
