// Draws: textured sprite quads (4-vertex triangle strip per instance) for a parallax stack: full-screen layers scroll their uv, sprites move with the camera.
// Inputs: location 0 transform vec4 @0 (centre xy px, half size xy px), 1 uv_rect vec4 @16 (uv origin xy, uv size xy), 2 layer vec4 @32 (position parallax, uv parallax, rotation rad, alpha); per-instance stream, stride 48.
// Uniforms (std140, binding 1): +0 vec4 camera (scroll x px, scroll y px, 2/width, 2/height).
#version 450

layout(location = 0) in vec4 in_transform;
layout(location = 1) in vec4 in_uv_rect;
layout(location = 2) in vec4 in_layer;

layout(std140, set = 0, binding = 1) uniform Camera {
    vec4 camera;
};

layout(location = 0) out vec2 out_uv;
layout(location = 1) out float out_alpha;

void main() {
    vec2 corner = vec2(gl_VertexIndex & 1, gl_VertexIndex >> 1);
    vec2 local = (corner * 2.0 - 1.0) * in_transform.zw;
    float c = cos(in_layer.z);
    float s = sin(in_layer.z);
    vec2 rotated = vec2(local.x * c - local.y * s, local.x * s + local.y * c);
    vec2 pixel = in_transform.xy + rotated - camera.xy * in_layer.x;
    vec2 uv_per_pixel = in_uv_rect.zw / (2.0 * in_transform.zw);
    out_uv = in_uv_rect.xy + corner * in_uv_rect.zw + camera.xy * in_layer.y * uv_per_pixel;
    out_alpha = in_layer.w;
    gl_Position = vec4(pixel * camera.zw - 1.0, 0.0, 1.0);
}
