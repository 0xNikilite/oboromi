#version 450
layout(location=0) in vec3 position;
layout(location=1) in vec3 normal;
layout(location=2) in vec2 uv;
layout(location=3) in vec3 tint;
layout(location=4) in float material;
layout(set=0,binding=0,std140) uniform Frame {
    mat4 vp; mat4 reflected_vp; vec4 eye_time; vec4 right_quality; vec4 up_rt;
    vec4 river; vec4 moon; vec4 settings;
};
layout(location=0) out vec3 world;
layout(location=1) out vec3 norm;
layout(location=2) out vec2 tex;
layout(location=3) out vec3 colour;
layout(location=4) flat out int kind;
void main() {
    world=position; norm=normal; tex=uv; colour=tint; kind=int(material);
    gl_Position=vp*vec4(position,1);
}
