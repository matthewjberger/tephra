use crate::vulkan::{CommandPool, GeometryBuffer};
use ash::vk;

pub struct ObjModel {
    pub buffers: GeometryBuffer,
    pub number_of_indices: u32,
}

impl ObjModel {
    pub fn new(command_pool: &CommandPool) -> Self {
        let (models, _) =
            tobj::load_obj("core/assets/models/teapot.obj", false).expect("Failed to load file");
        let vertices = &models[0].mesh.positions;
        let indices = &models[0].mesh.indices;
        let number_of_indices = indices.len() as u32;
        let buffers = GeometryBuffer::new(command_pool, vertices, Some(indices));
        Self {
            buffers,
            number_of_indices,
        }
    }

    pub fn create_vertex_attributes() -> [vk::VertexInputAttributeDescription; 1] {
        let position_description = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(0)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset(0)
            .build();

        [position_description]
    }

    pub fn create_vertex_input_descriptions() -> [vk::VertexInputBindingDescription; 1] {
        let vertex_input_binding_description = vk::VertexInputBindingDescription::builder()
            .binding(0)
            .stride((3 * std::mem::size_of::<f32>()) as _)
            .input_rate(vk::VertexInputRate::VERTEX)
            .build();
        [vertex_input_binding_description]
    }
}
