#version 450
layout(set=0,binding=0,std140) uniform Frame {
    mat4 vp; mat4 reflected_vp; vec4 eye_time; vec4 right_quality; vec4 up_rt;
    vec4 river; vec4 moon; vec4 settings;
};
struct Cone { vec4 origin_height; vec4 radius_tilt; };
layout(set=0,binding=1,std430) readonly buffer Cones { Cone cones[]; };
layout(location=0) out vec3 world;
layout(location=1) out vec3 norm;
layout(location=2) out vec2 tex;
layout(location=3) out vec3 colour;
layout(location=4) flat out int kind;
vec3 point(float u,float t,Cone c) {
    float angle=u*6.2831853;
    float radius=(pow(1.-t,1.4)+.006)*(1.+.08*sin(angle*5.+t*8.))*c.radius_tilt.x;
    return c.origin_height.xyz+vec3(radius*cos(angle)+t*c.radius_tilt.y,t*c.origin_height.w,radius*sin(angle)+t*c.radius_tilt.z);
}
void main() {
    const ivec2 corners[6]=ivec2[](ivec2(0,0),ivec2(1,0),ivec2(0,1),ivec2(1,0),ivec2(1,1),ivec2(0,1));
    int cell=gl_VertexIndex/6;ivec2 p=ivec2(cell%24,cell/24)+corners[gl_VertexIndex%6];
    float u=float(p.x)/24.,t=float(p.y)/12.;Cone c=cones[gl_InstanceIndex];
    world=point(u,t,c);
    vec3 a=point(u+.001,t,c)-point(u-.001,t,c),b=point(u,min(t+.001,1.),c)-point(u,max(t-.001,0.),c);
    norm=normalize(cross(b,a))*sign(c.origin_height.w);
    tex=vec2(u*1.3,t*abs(c.origin_height.w)*.45);
    colour=vec3(.65,.69,.62)*mix(.35,1.,smoothstep(0.,.15,t));kind=1;
    gl_Position=vp*vec4(world,1);
}
