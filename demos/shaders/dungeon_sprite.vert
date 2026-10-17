#version 450
layout(set=0,binding=0,std140) uniform Frame {
    mat4 vp; mat4 reflected_vp; vec4 eye_time; vec4 right_quality; vec4 up_rt;
    vec4 river; vec4 moon; vec4 settings;
};
struct Particle { vec4 position; vec4 size; vec4 colour; };
layout(set=0,binding=1,std430) readonly buffer Particles { Particle particles[]; };
layout(location=0) out vec2 uv;
layout(location=1) out vec4 colour;
layout(location=2) flat out vec2 kind_seed;
layout(location=3) out vec3 world;
void main(){
    const vec2 corners[6]=vec2[](vec2(-1,-1),vec2(1,-1),vec2(-1,1),vec2(-1,1),vec2(1,-1),vec2(1,1));
    Particle p=particles[gl_InstanceIndex]; vec2 c=corners[gl_VertexIndex];
    if((uint(moon.w)&1u)!=0u && p.position.w<3.){gl_Position=vec4(2,2,2,1);return;}
    vec3 vertical=p.position.w==0.?vec3(0,1,0):up_rt.xyz;
    vec3 horizontal=right_quality.xyz;
    if(p.position.w==5. || p.position.w==7.) { horizontal=vec3(1,0,0);vertical=vec3(0,0,1); }
    if(p.position.w==9.) {horizontal=vec3(sin(p.size.z),0,cos(p.size.z));vertical=vec3(0,1,0);}
    world=p.position.xyz+horizontal*c.x*p.size.x+vertical*c.y*p.size.y;
    gl_Position=vp*vec4(world,1);uv=c;colour=p.colour;kind_seed=vec2(p.position.w,p.size.z);
}
