// Draws: one sprite texel per fragment from the layer atlas, alpha-blended (SRC_ALPHA, ONE_MINUS_SRC_ALPHA) back to front.
// Inputs: location 0 uv vec2 (atlas, repeat wrap so tiles scroll), 1 alpha float; sampler at binding 2 (the atlas, nearest filter for crisp pixels).
// Uniforms (std140, binding 1): +0 vec4 tint (rgb multiplier, a = global alpha).
#version 450

layout(location = 0) in vec2 in_uv;
layout(location = 1) in float in_alpha;

layout(std140, set = 0, binding = 1) uniform Sprite {
    vec4 tint;
};

layout(set = 0, binding = 2) uniform sampler2D atlas;

layout(location = 0) out vec4 out_colour;

void main() {
    vec4 texel = texture(atlas, in_uv);
    out_colour = vec4(texel.rgb * tint.rgb, texel.a * in_alpha * tint.a);
}
