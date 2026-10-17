// Draws: a glTF mesh (BoomBox, Avocado) in world space for pbr.frag, one vertex per index.
// Inputs: location 0 position vec3 @0, 1 normal vec3 @12, 2 tangent vec4 @24 (w = bitangent sign), 3 uv vec2 @40; stride 48.
// Uniforms (std140, binding 1): +0 mat4 model, +64 mat4 view_proj; model must be rotation * uniform scale.
#version 450

layout(location = 0) in vec3 in_position;
layout(location = 1) in vec3 in_normal;
layout(location = 2) in vec4 in_tangent;
layout(location = 3) in vec2 in_uv;

layout(std140, set = 0, binding = 1) uniform Vertex {
    mat4 model;
    mat4 view_proj;
};

layout(location = 0) out vec3 out_world;
layout(location = 1) out vec3 out_normal;
layout(location = 2) out vec4 out_tangent;
layout(location = 3) out vec2 out_uv;

void main() {
    vec4 world = model * vec4(in_position, 1.0);
    mat3 linear = mat3(model);
    out_world = world.xyz;
    out_normal = linear * in_normal;
    out_tangent = vec4(linear * in_tangent.xyz, in_tangent.w);
    out_uv = in_uv;
    gl_Position = view_proj * world;
}
