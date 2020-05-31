#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

layout (location = 0) in vec3 inPos;
layout (location = 1) in vec3 inNormal;
layout (location = 2) in vec2 inUV0;
layout (location = 3) in vec2 inUV1;
layout (location = 4) in vec4 inJoint0;
layout (location = 5) in vec4 inWeight0;

#define MAX_NUM_JOINTS 128

layout(binding = 0) uniform UboView {
  mat4 view;
  mat4 projection;
  vec4 cameraPosition;
  mat4 jointMatrices[MAX_NUM_JOINTS];
} uboView;

layout(binding = 1) uniform UboInstance {
  mat4 model;
  float jointCount;
  float jointOffset;
  float outlineWidth;
} uboInstance;

void main()
{
  mat4 skinMatrix = mat4(1.0);
  if (uboInstance.jointCount > 0.0) {
    skinMatrix =
      inWeight0.x * uboView.jointMatrices[int(inJoint0.x + uboInstance.jointOffset)] +
      inWeight0.y * uboView.jointMatrices[int(inJoint0.y + uboInstance.jointOffset)] +
      inWeight0.z * uboView.jointMatrices[int(inJoint0.z + uboInstance.jointOffset)] +
      inWeight0.w * uboView.jointMatrices[int(inJoint0.w + uboInstance.jointOffset)];
  }
  vec4 locPos = uboInstance.model * skinMatrix * vec4(inPos.xyz + inNormal * uboInstance.outlineWidth, 1.0);
  locPos.y = -locPos.y;
  gl_Position =  uboView.projection * uboView.view * vec4(outWorldPos, 1.0);
}
