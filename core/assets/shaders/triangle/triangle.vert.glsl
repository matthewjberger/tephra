#version 450

layout (location = 0) in vec3 inPos;

layout(binding = 0) uniform Ubo {
  mat4 model;
  mat4 view;
  mat4 projection;
} ubo;

void main()
{
  gl_Position =  ubo.projection * ubo.view * ubo.model * vec4(inPos, 1.0);
}

