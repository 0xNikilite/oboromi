// Draws: pbr.frag's metallic-roughness shading with ray-queried shadows, one shadow ray per pixel towards the light and one reflection ray for metallic pixels.
// Inputs: pbr.vert's outputs; samplers at bindings 2-6 as pbr.frag; top-level acceleration structure at binding 7, built in world space (BoomBox instance with the frame's model matrix, ground plane).
// Uniforms (std140, binding 1): pbr.frag's block plus +112 vec4 sky (rgb reflected on a miss, w = shadow ray bias in world units).
#version 460
#extension GL_EXT_ray_query : require

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
    vec4 sky;
};

layout(set = 0, binding = 2) uniform sampler2D base_colour_map;
layout(set = 0, binding = 3) uniform sampler2D normal_map;
layout(set = 0, binding = 4) uniform sampler2D metallic_roughness_map;
layout(set = 0, binding = 5) uniform sampler2D occlusion_map;
layout(set = 0, binding = 6) uniform sampler2D emissive_map;
layout(set = 0, binding = 7) uniform accelerationStructureEXT scene;

layout(location = 0) out vec4 out_colour;

const float PI = 3.14159265;

vec3 srgb_to_linear(vec3 c) {
    return pow(c, vec3(2.2));
}

vec3 aces(vec3 x) {
    return clamp(x * (2.51 * x + 0.03) / (x * (2.43 * x + 0.59) + 0.14), 0.0, 1.0);
}

bool occluded(vec3 origin, vec3 dir, float t_max, uint flags) {
    rayQueryEXT q;
    rayQueryInitializeEXT(q, scene, gl_RayFlagsOpaqueEXT | flags, 0xff, origin, sky.w, dir, t_max);
    while (rayQueryProceedEXT(q)) {}
    return rayQueryGetIntersectionTypeEXT(q, true) != gl_RayQueryCommittedIntersectionNoneEXT;
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
    vec3 geometric = normalize(in_normal);
    vec3 t = normalize(in_tangent.xyz - geometric * dot(geometric, in_tangent.xyz));
    vec3 n = normalize(mat3(t, cross(geometric, t) * in_tangent.w, geometric) * tangent_normal);

    vec3 v = normalize(camera_pos.xyz - in_world);
    vec3 l = normalize(light_dir.xyz);
    vec3 h = normalize(v + l);
    float n_l = max(dot(n, l), 0.0);
    float n_v = max(dot(n, v), 1e-4);
    float n_h = max(dot(n, h), 0.0);
    float a2 = roughness * roughness * roughness * roughness;
    float d = n_h * n_h * (a2 - 1.0) + 1.0;
    float k = (roughness + 1.0) * (roughness + 1.0) / 8.0;
    float visibility = 1.0 / ((n_l * (1.0 - k) + k) * (n_v * (1.0 - k) + k) * 4.0);
    vec3 f0 = mix(vec3(0.04), albedo, metallic);
    vec3 fresnel = f0 + (1.0 - f0) * pow(1.0 - max(dot(v, h), 0.0), 5.0);

    vec3 origin = in_world + geometric * sky.w;
    float lit = n_l > 0.0 && !occluded(origin, l, 1e4, gl_RayFlagsTerminateOnFirstHitEXT) ? 1.0 : 0.0;
    vec3 direct = ((1.0 - fresnel) * (1.0 - metallic) * albedo / PI + fresnel * a2 / (PI * d * d) * visibility) * light_colour.rgb * n_l * lit;
    vec3 indirect = ambient.rgb * albedo * occlusion;
    if (metallic > 0.5) {
        vec3 r = reflect(-v, n);
        indirect += fresnel * (occluded(origin, r, 1e4, 0u) ? ambient.rgb : sky.rgb) * (1.0 - roughness) * occlusion;
    }
    out_colour = vec4(pow(aces(direct + indirect + emissive), vec3(1.0 / 2.2)), 1.0);
}
