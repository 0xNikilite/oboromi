// Draws: the first pass's colour target with time-driven radial chromatic aberration, rolling scanlines and a vignette.
// Inputs: location 0 uv vec2; sampler at binding 2 (the scene colour target, clamp to edge, linear).
// Uniforms (std140, binding 1): +0 vec4 effect (time s, aberration px, scanline strength, vignette strength), +16 vec4 size (width, height, 1/width, 1/height).
#version 450

layout(location = 0) in vec2 in_uv;

layout(std140, set = 0, binding = 1) uniform Post {
    vec4 effect;
    vec4 size;
};

layout(set = 0, binding = 2) uniform sampler2D scene;

layout(location = 0) out vec4 out_colour;

void main() {
    float time = effect.x;
    vec2 centred = in_uv - 0.5;
    float radius2 = dot(centred, centred);

    float pulse = 0.75 + 0.25 * sin(time * 1.7);
    vec2 shift = centred * radius2 * effect.y * pulse * size.zw * 4.0;
    float r = texture(scene, in_uv + shift).r;
    float g = texture(scene, in_uv).g;
    float b = texture(scene, in_uv - shift).b;
    vec3 colour = vec3(r, g, b);

    float row = in_uv.y * size.y;
    float scan = 0.5 + 0.5 * sin(row * 3.14159265);
    float roll = 0.5 + 0.5 * sin(in_uv.y * 6.0 - time * 2.0);
    colour *= 1.0 - effect.z * (1.0 - scan) * (0.6 + 0.4 * roll);

    float vignette = 1.0 - effect.w * smoothstep(0.1, 0.5, radius2);
    out_colour = vec4(colour * vignette, 1.0);
}
