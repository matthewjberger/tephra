#version 450

layout (location = 0) in vec3 inPos;
layout (location = 1) in vec3 inNormal;

layout(binding = 0) uniform Ubo {
  mat4 model;
  mat4 view;
  mat4 projection;
  float outlineWidth;
} ubo;

void main()
{
  vec3 pos = inPos;
  pos.y *= -1.0;
  gl_Position =  ubo.projection * ubo.view * ubo.model * vec4(pos.xyz + inNormal * ubo.outlineWidth, 1.0);
}

