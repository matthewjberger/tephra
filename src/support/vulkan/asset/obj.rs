use crate::vulkan::{CommandPool, GeometryBuffer};
use ash::vk;

pub struct ObjModel {
    pub buffers: GeometryBuffer,
}

impl ObjModel {
    pub fn new(command_pool: &CommandPool, path: &str) -> Self {
        let (models, _) = tobj::load_obj(path, false).expect("Failed to load file");

        let mut vertices = Vec::new();
        let mesh = &models[0].mesh;
        for index in 0..mesh.positions.len() / 3 {
            vertices.extend_from_slice(&[
                mesh.positions[3 * index],
                mesh.positions[3 * index + 1],
                mesh.positions[3 * index + 2],
            ]);

            if mesh.normals.is_empty() {
                vertices.extend_from_slice(&[0.0, 0.0, 0.0]);
            } else {
                vertices.extend_from_slice(&[
                    mesh.normals[3 * index],
                    mesh.normals[3 * index + 1],
                    mesh.normals[3 * index + 2],
                ]);
            }

            if mesh.texcoords.is_empty() {
                vertices.extend_from_slice(&[0.0, 0.0]);
            } else {
                vertices
                    .extend_from_slice(&[mesh.texcoords[2 * index], mesh.texcoords[2 * index + 1]]);
            }
        }

        let indices = &models[0].mesh.indices;
        let buffers = GeometryBuffer::new(command_pool, &vertices, Some(indices));
        Self { buffers }
    }

    pub fn create_vertex_attributes() -> [vk::VertexInputAttributeDescription; 3] {
        let float_size = std::mem::size_of::<f32>();
        let position_description = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(0)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset(0)
            .build();

        let normal_description = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(1)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset((3 * float_size) as _)
            .build();

        let texcoord_description = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(2)
            .format(vk::Format::R32G32_SFLOAT)
            .offset((6 * float_size) as _)
            .build();

        [
            position_description,
            normal_description,
            texcoord_description,
        ]
    }

    pub fn create_vertex_input_descriptions() -> [vk::VertexInputBindingDescription; 1] {
        let vertex_input_binding_description = vk::VertexInputBindingDescription::builder()
            .binding(0)
            .stride((8 * std::mem::size_of::<f32>()) as _)
            .input_rate(vk::VertexInputRate::VERTEX)
            .build();
        [vertex_input_binding_description]
    }
}
