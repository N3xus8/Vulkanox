use std::io::Cursor;
use std::sync::Arc;

use vulkano::buffer::{Buffer, BufferCreateInfo, BufferUsage};
use vulkano::command_buffer::allocator::StandardCommandBufferAllocator;
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, BlitImageInfo, BufferImageCopy, ClearColorImageInfo,
    CommandBufferUsage, CopyBufferToImageInfo, CopyImageInfo, ImageBlit, ImageCopy,
    PrimaryAutoCommandBuffer,
};
use vulkano::device::{Device, Queue};
use vulkano::format::Format;
use vulkano::image::sampler::{
    Filter, Sampler, SamplerAddressMode, SamplerCreateInfo, SamplerMipmapMode,
};
use vulkano::image::view::ImageView;
use vulkano::image::{
    Image, ImageAspects, ImageCreateInfo, ImageLayout, ImageSubresourceLayers, ImageType,
    ImageUsage,
};
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryTypeFilter, StandardMemoryAllocator};
use vulkano::DeviceSize;

use crate::{error::Result, utils::read_file_to_bytes};

pub fn create_texture(
    path: &str,
    command_builder: &mut AutoCommandBufferBuilder<
        PrimaryAutoCommandBuffer<Arc<StandardCommandBufferAllocator>>,
        Arc<StandardCommandBufferAllocator>,
    >,
    memory_allocator: Arc<StandardMemoryAllocator>,
) -> Result<Arc<ImageView>> {
    // load the image data and dimensions before event loop
    let texture = {
        //  to read in the texture file as bytes
        let png_bytes = read_file_to_bytes(path);
        // Cursor wraps a byte array to provide an implementation of the Read trait.
        // This lets us pass it into the Decoder provided by the png crate.
        let cursor = Cursor::new(png_bytes);
        let decoder = png::Decoder::new(cursor);

        // This gives us some information about the png image.
        // as well as a new Reader to actually load the source bytes into a vector.
        let mut reader = decoder.read_info().expect("error png reader");

        let info = reader.info();

        let img_size = [info.width, info.height];
        // These are the image dimensions we’ll pass along to Vulkan when we create the texture.
        let extent = [info.width  , info.height , 1];

        let mut mip_width = info.width;
        let mut mip_height = info.height;

        // Mip level for mipmap
        // This calculates the number of levels in the mip chain.
        // The max method selects the largest dimension.
        // The log2 method calculates how many times that dimension can be divided by 2.
        //The floor method handles cases where the largest dimension is not a power of 2.
        // 1 is added so that the original image has a mip level.
        let mip_levels = (info.width.max(info.height) as f32).log2().floor() as u32 + 1;
        println!("Mip levels: {mip_levels:}");
        // This is how we actually load the image into a Rust vector.
        // The specific call to reader.next_frame is because a png file can have multiple “frames”.

        let depth: u32 = match info.bit_depth {
            png::BitDepth::One => 1,
            png::BitDepth::Two => 2,
            png::BitDepth::Four => 4,
            png::BitDepth::Eight => 8,
            png::BitDepth::Sixteen => 16,
        };

        let upload_buffer = Buffer::new_slice(
            memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::TRANSFER_SRC,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_HOST
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            (info.width * info.height * depth) as DeviceSize,
        )?;

        reader.next_frame(&mut upload_buffer.write()?)?;

        let image = Image::new(
            memory_allocator.clone(),
            ImageCreateInfo {
                format: Format::R8G8B8A8_SRGB,
                extent,
                usage: ImageUsage::TRANSFER_SRC | ImageUsage::TRANSFER_DST | ImageUsage::SAMPLED,
                mip_levels,
                ..Default::default()
            },
            AllocationCreateInfo::default(),
        )?;
        command_builder
                // Clear the image buffer.
                .clear_color_image(ClearColorImageInfo::image(image.clone()))
                .unwrap()
                // Put our image in the top left corner.
                .copy_buffer_to_image(CopyBufferToImageInfo {
                    regions: [BufferImageCopy {
                        image_subresource: image.subresource_layers(),
                        image_extent: [img_size[0], img_size[1], 1],
                        ..Default::default()
                    }]
                    .into(),
                    ..CopyBufferToImageInfo::buffer_image(upload_buffer, image.clone()) // upload Image here
                })?;
                // Copy from the top left corner to the bottom right corner.
/*                 .copy_image(CopyImageInfo {
                    // Copying within the same image requires the General layout if the source and
                    // destination subresources overlap.
                    src_image_layout: ImageLayout::General,
                    dst_image_layout: ImageLayout::General,
                    regions: [ImageCopy {
                        src_subresource: image.subresource_layers(),
                        src_offset: [0, 0, 0],
                        dst_subresource: image.subresource_layers(),
                        dst_offset: [img_size[0], img_size[1], 0],
                        extent: [img_size[0], img_size[1], 1],
                        ..Default::default()
                    }]
                    .into(),
                    ..CopyImageInfo::images(image.clone(), image.clone())
                })?; */
        // MIPMAP
        for level in 1..mip_levels {
            let src_subresource = ImageSubresourceLayers {
                mip_level: level - 1,
                array_layers: 0..1,
                aspects: ImageAspects::COLOR,
            };

            let dst_subresource = ImageSubresourceLayers {
                mip_level: level,
                array_layers: 0..1,
                aspects: ImageAspects::COLOR,
            };

            let src_offsets = [[0, 0, 0], [mip_width, mip_height, 1]];
            let dst_offsets = [
                [0, 0, 0],
                [
                    (if mip_width > 1 { mip_width / 2 } else { 1 }),
                    (if mip_height > 1 { mip_height / 2 } else { 1 }),
                    1,
                ],
            ];
            println!("DEBUG --> src: {:?} ; dst {:?}", src_offsets, dst_offsets);
            let blit = ImageBlit {
                src_subresource,
                src_offsets,
                dst_subresource,
                dst_offsets,
                ..Default::default()
            };

            // Here, we perform image copying and blitting on the same image.
            command_builder
                .blit_image(BlitImageInfo {
                    src_image_layout: ImageLayout::TransferSrcOptimal,
                    dst_image_layout: ImageLayout::TransferDstOptimal,
                    regions: [blit].into(),
                    filter: Filter::Linear,
                    ..BlitImageInfo::images(image.clone(), image.clone())
                })?;

            if mip_width > 1 {
                mip_width /= 2;
            }

            if mip_height > 1 {
                mip_height /= 2;
            }
        }
        ImageView::new_default(image)?
    };

    Ok(texture)
}

pub fn create_sampler(device: Arc<Device>) -> Result<Arc<Sampler>> {
    let sampler = Sampler::new(
        device.clone(),
        SamplerCreateInfo {
            mag_filter: Filter::Linear,
            min_filter: Filter::Linear,
            mipmap_mode: SamplerMipmapMode::Nearest,
            address_mode: [SamplerAddressMode::Repeat; 3],
            mip_lod_bias: 0.0,
            ..Default::default()
        },
    )?;

    Ok(sampler)
}
