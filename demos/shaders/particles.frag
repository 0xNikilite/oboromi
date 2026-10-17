// Draws: a soft round glow per quad for additive blending (ONE, ONE), so overlapping particles brighten.
// Inputs: location 0 corner vec2 (-1..1 across the quad), 1 colour vec4 (rgb, a = brightness).
// Uniforms: none.
#version 450

layout(location = 0) in vec2 in_corner;
layout(location = 1) in vec4 in_colour;

layout(location = 0) out vec4 out_colour;

void main() {
    float falloff = max(1.0 - dot(in_corner, in_corner), 0.0);
    float glow = falloff * falloff * in_colour.a;
    out_colour = vec4(in_colour.rgb * glow, glow);
}
