use crate::vulkan::{CommandPool, VulkanContext};
use ash::{version::DeviceV1_0, vk};
use snafu::{ResultExt, Snafu};
use std::sync::Arc;

type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, Snafu)]
#[snafu(visibility = "pub(crate)")]
pub enum Error {
    #[snafu(display("Failed to create buffer: {}", source))]
    CreateBuffer { source: vk_mem::error::Error },

    #[snafu(display("Failed to map memory: {}", source))]
    MapMemory { source: vk_mem::error::Error },

    #[snafu(display("Failed to unmap memory: {}", source))]
    UnmapMemory { source: vk_mem::error::Error },
}

pub struct Buffer {
    buffer: vk::Buffer,
    allocation: vk_mem::Allocation,
    allocation_info: vk_mem::AllocationInfo,
    context: Arc<VulkanContext>,
}

impl Buffer {
    pub fn new(
        context: Arc<VulkanContext>,
        allocation_create_info: &vk_mem::AllocationCreateInfo,
        buffer_create_info: &vk::BufferCreateInfo,
    ) -> Result<Self> {
        let (buffer, allocation, allocation_info) = context
            .allocator()
            .create_buffer(&buffer_create_info, &allocation_create_info)
            .context(CreateBuffer {})?;

        let buffer = Self {
            buffer,
            allocation,
            allocation_info,
            context,
        };

        Ok(buffer)
    }

    pub fn new_mapped_basic(
        context: Arc<VulkanContext>,
        size: vk::DeviceSize,
        buffer_usage: vk::BufferUsageFlags,
        memory_usage: vk_mem::MemoryUsage,
    ) -> Result<Self> {
        let allocation_create_info = vk_mem::AllocationCreateInfo {
            usage: memory_usage,
            ..Default::default()
        };

        let buffer_create_info = vk::BufferCreateInfo::builder()
            .size(size)
            .usage(buffer_usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .build();

        Buffer::new(context, &allocation_create_info, &buffer_create_info)
    }

    pub fn upload_to_buffer<T>(&self, data: &[T], offset: usize) -> Result<()> {
        // TODO: Add checks for size of data being written
        let data_pointer = self.map_memory().context(MapMemory {})?;
        unsafe {
            data_pointer.add(offset);
            (data_pointer as *mut T).copy_from_nonoverlapping(data.as_ptr(), data.len());
        }
        self.unmap_memory().context(UnmapMemory {})
    }

    pub fn upload_to_buffer_aligned<T: Copy>(
        &self,
        data: &[T],
        offset: usize,
        alignment: vk::DeviceSize,
    ) -> Result<()> {
        let data_pointer = self.map_memory().context(MapMemory {})?;
        unsafe {
            let mut align = ash::util::Align::new(
                data_pointer.add(offset) as _,
                alignment,
                self.allocation_info.get_size() as _,
            );
            align.copy_from_slice(data);
        }
        self.unmap_memory().context(UnmapMemory {})
    }

    pub fn map_memory(&self) -> vk_mem::error::Result<*mut u8> {
        self.context.allocator().map_memory(&self.allocation)
    }

    pub fn unmap_memory(&self) -> vk_mem::error::Result<()> {
        self.context.allocator().unmap_memory(&self.allocation)
    }

    pub fn flush(&self, offset: usize, size: usize) -> vk_mem::error::Result<()> {
        self.context
            .allocator()
            .flush_allocation(&self.allocation, offset, size)
    }

    pub fn buffer(&self) -> vk::Buffer {
        self.buffer
    }

    pub fn allocation(&self) -> &vk_mem::Allocation {
        &self.allocation
    }

    pub fn allocation_info(&self) -> &vk_mem::AllocationInfo {
        &self.allocation_info
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        self.context
            .allocator()
            .destroy_buffer(self.buffer, &self.allocation)
            .expect("Failed to destroy buffer!");
    }
}

pub struct GeometryBuffer {
    pub vertex_buffer: Buffer,
    pub index_buffer: Option<Buffer>,
    pub number_of_indices: u32,
}

impl GeometryBuffer {
    pub fn new(command_pool: &CommandPool, vertices: &[f32], indices: Option<&[u32]>) -> Self {
        let vertex_buffer =
            Self::create_buffer(command_pool, &vertices, vk::BufferUsageFlags::VERTEX_BUFFER);

        let mut number_of_indices = 0;
        let index_buffer = if let Some(indices) = indices {
            number_of_indices = indices.len() as u32;
            let index_buffer =
                Self::create_buffer(command_pool, &indices, vk::BufferUsageFlags::INDEX_BUFFER);
            Some(index_buffer)
        } else {
            None
        };

        Self {
            vertex_buffer,
            index_buffer,
            number_of_indices,
        }
    }

    fn create_buffer<T: Copy>(
        command_pool: &CommandPool,
        data: &[T],
        usage_flags: vk::BufferUsageFlags,
    ) -> Buffer {
        let region = vk::BufferCopy {
            src_offset: 0,
            dst_offset: 0,
            size: (data.len() * std::mem::size_of::<T>()) as ash::vk::DeviceSize,
        };
        command_pool.create_device_local_buffer(usage_flags, &data, &[region])
    }

    pub fn bind(&self, device: &ash::Device, command_buffer: vk::CommandBuffer) {
        let offsets = [0];
        let vertex_buffers = [self.vertex_buffer.buffer()];

        unsafe {
            device.cmd_bind_vertex_buffers(command_buffer, 0, &vertex_buffers, &offsets);

            if let Some(index_buffer) = self.index_buffer.as_ref() {
                device.cmd_bind_index_buffer(
                    command_buffer,
                    index_buffer.buffer(),
                    0,
                    vk::IndexType::UINT32,
                );
            }
        }
    }
}
