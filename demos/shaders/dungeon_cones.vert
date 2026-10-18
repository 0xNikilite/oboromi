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
void main() {
    const ivec2 corners[6]=ivec2[](ivec2(0,0),ivec2(1,0),ivec2(0,1),ivec2(1,0),ivec2(1,1),ivec2(0,1));
    int sides=right_quality.w>.5?24:12, rings=right_quality.w>.5?12:8;
    int cell=gl_VertexIndex/6;ivec2 p=ivec2(cell%sides,cell/sides)+corners[gl_VertexIndex%6];
    float u=float(p.x)/float(sides),t=float(p.y)/float(rings);Cone c=cones[gl_InstanceIndex];
    float h=t*(2.-t),angle=u*6.2831853,cap=sqrt(max(.000004,1.-h*h));
    float taper=exp(-h*4.),profile=.55+.45*taper,phase=angle*5.+h*8.+c.origin_height.x;
    float variation=1.+.12*sin(phase),radius=cap*profile*variation*c.radius_tilt.x;
    float drdh=((-h/cap*profile-cap*1.8*taper)*variation+cap*profile*.96*cos(phase))*c.radius_tilt.x;
    float drda=cap*profile*.6*cos(phase)*c.radius_tilt.x;
    vec2 radial=vec2(cos(angle),sin(angle));
    world=c.origin_height.xyz+vec3(radius*radial.x+h*c.radius_tilt.y,h*c.origin_height.w,radius*radial.y+h*c.radius_tilt.z);
    vec3 a=vec3(drda*radial.x-radius*radial.y,0,drda*radial.y+radius*radial.x);
    vec3 b=vec3(drdh*radial.x+c.radius_tilt.y,c.origin_height.w,drdh*radial.y+c.radius_tilt.z);
    norm=normalize(cross(b,a))*sign(c.origin_height.w);
    tex=vec2(u*1.3,t);
    colour=vec3(1.45,1.32,1.08);kind=7;
    gl_Position=vp*vec4(world,1);
}
