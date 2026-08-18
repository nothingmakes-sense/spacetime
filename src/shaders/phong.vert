#version 450
// Reference GLSL. The live shader is phong.wgsl (compiled by build.rs).

layout(location = 0) in vec3 inPosition;
layout(location = 1) in vec3 inNormal;
layout(location = 2) in vec2 inUV;

layout(set = 0, binding = 0) uniform CameraUBO {
    mat4 view;
    mat4 proj;
    vec3 cameraPos;
} camera;

layout(push_constant) uniform PushConstants {
    mat4 model;
} pc;

layout(location = 0) out vec3 fragPos;
layout(location = 1) out vec3 fragNormal;
layout(location = 2) out vec2 fragUV;

void main() {
    vec4 worldPos = pc.model * vec4(inPosition, 1.0);
    fragPos     = worldPos.xyz;
    fragNormal  = mat3(pc.model) * inNormal;
    fragUV      = inUV;
    gl_Position = camera.proj * camera.view * worldPos;
}
