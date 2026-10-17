// Draws: one camera-facing quad per particle instance, 4-vertex triangle strip, corners from gl_VertexIndex.
// Inputs: location 0 position vec4 @0 (xyz, w = age 0..1), 1 colour vec4 @32; per-instance stream, stride 48 (the particles_update.comp buffer).
// Uniforms (std140, binding 1): +0 mat4 view_proj, +64 vec4 camera_right (xyz, w = quad half size), +80 vec4 camera_up.
#version 450

layout(location = 0) in vec4 in_position;
layout(location = 1) in vec4 in_colour;

layout(std140, set = 0, binding = 1) uniform Particles {
    mat4 view_proj;
    vec4 camera_right;
    vec4 camera_up;
};

layout(location = 0) out vec2 out_corner;
layout(location = 1) out vec4 out_colour;

void main() {
    vec2 corner = vec2(gl_VertexIndex & 1, gl_VertexIndex >> 1) * 2.0 - 1.0;
    float size = camera_right.w * (1.0 - 0.5 * in_position.w);
    vec3 world = in_position.xyz + (camera_right.xyz * corner.x + camera_up.xyz * corner.y) * size;
    out_corner = corner;
    out_colour = in_colour;
    gl_Position = view_proj * vec4(world, 1.0);
}
