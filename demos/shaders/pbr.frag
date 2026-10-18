// Draws: glTF metallic-roughness shading, one directional light plus ambient, ACES-fitted tone map, gamma 2.2 out to an RGBA8 UNORM target.
// Inputs: location 0 world position vec3, 1 normal vec3, 2 tangent vec4, 3 uv vec2; samplers at bindings 2-6 (baseColor, normal, metallicRoughness, occlusion, emissive).
// Uniforms (std140, binding 1): +0 vec4 light_dir (towards light), +16 vec4 light_colour (rgb * intensity), +32 vec4 ambient, +48 vec4 camera_pos, +64 vec4 base_colour_factor, +80 vec4 emissive_factor, +96 vec4 factors (metallic, roughness, normal scale, occlusion strength).
#version 450

layout(location = 0) in vec3 in_world;
layout(location = 1) in vec3 in_normal;
layout(location = 2) in vec4 in_tangent;
layout(location = 3) in vec2 in_uv;

layout(std140, set = 0, binding = 1) uniform Fragment {
    vec4 light_dir;
    vec4 light_colour;
    vec4 ambient;
    vec4 camera_pos;
    vec4 base_colour_factor;
    vec4 emissive_factor;
    vec4 factors;
};

layout(set = 0, binding = 2) uniform sampler2D base_colour_map;
layout(set = 0, binding = 3) uniform sampler2D normal_map;
layout(set = 0, binding = 4) uniform sampler2D metallic_roughness_map;
layout(set = 0, binding = 5) uniform sampler2D occlusion_map;
layout(set = 0, binding = 6) uniform sampler2D emissive_map;

layout(location = 0) out vec4 out_colour;

const float PI = 3.14159265;

// The textures are bound as UNORM (the TIC sRGB bit is not decoded yet), so sRGB decode happens here.
vec3 srgb_to_linear(vec3 c) {
    return pow(c, vec3(2.2));
}

vec3 aces(vec3 x) {
    return clamp(x * (2.51 * x + 0.03) / (x * (2.43 * x + 0.59) + 0.14), 0.0, 1.0);
}

void main() {
    vec4 base = texture(base_colour_map, in_uv);
    vec3 albedo = srgb_to_linear(base.rgb) * base_colour_factor.rgb;
    vec2 mr = texture(metallic_roughness_map, in_uv).bg * factors.xy;
    float metallic = mr.x;
    float roughness = clamp(mr.y, 0.04, 1.0);
    float occlusion = mix(1.0, texture(occlusion_map, in_uv).r, factors.w);
    vec3 emissive = srgb_to_linear(texture(emissive_map, in_uv).rgb) * emissive_factor.rgb;

    vec3 tangent_normal = texture(normal_map, in_uv).xyz * 2.0 - 1.0;
    tangent_normal.xy *= factors.z;
    vec3 n = normalize(in_normal);
    vec3 t = normalize(in_tangent.xyz - n * dot(n, in_tangent.xyz));
    vec3 b = cross(n, t) * in_tangent.w;
    n = normalize(mat3(t, b, n) * tangent_normal);

    vec3 v = normalize(camera_pos.xyz - in_world);
    vec3 l = normalize(light_dir.xyz);
    vec3 h = normalize(v + l);
    float n_l = max(dot(n, l), 0.0);
    float n_v = max(dot(n, v), 1e-4);
    float n_h = max(dot(n, h), 0.0);
    float v_h = max(dot(v, h), 0.0);

    float a = roughness * roughness;
    float a2 = a * a;
    float d = n_h * n_h * (a2 - 1.0) + 1.0;
    float distribution = a2 / (PI * d * d);
    float k = (roughness + 1.0) * (roughness + 1.0) / 8.0;
    float visibility = 1.0 / ((n_l * (1.0 - k) + k) * (n_v * (1.0 - k) + k) * 4.0);
    vec3 f0 = mix(vec3(0.04), albedo, metallic);
    vec3 fresnel = f0 + (1.0 - f0) * pow(1.0 - v_h, 5.0);

    vec3 diffuse = (1.0 - fresnel) * (1.0 - metallic) * albedo / PI;
    vec3 specular = fresnel * distribution * visibility;
    vec3 direct = (diffuse + specular) * light_colour.rgb * n_l;
    vec3 fill = normalize(vec3(-l.x, 0.35, l.z));
    direct += diffuse * vec3(0.18, 0.28, 0.5) * max(dot(n, fill), 0.0);
    direct += vec3(0.32, 0.38, 0.5) * pow(1.0 - n_v, 4.0) * (0.15 + 0.35 * max(dot(n, -l), 0.0));
    vec3 indirect = ambient.rgb * albedo * occlusion;
    vec3 hdr = direct + indirect + emissive;
    out_colour = vec4(pow(aces(hdr), vec3(1.0 / 2.2)), 1.0);
}
