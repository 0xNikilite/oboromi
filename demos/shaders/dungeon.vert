#version 450
layout(location=0) in vec3 position;
layout(location=1) in vec3 normal;
layout(location=2) in vec2 uv;
layout(location=3) in vec3 tint;
layout(location=4) in float material;
layout(set=0,binding=0,std140) uniform Frame {
    mat4 vp; mat4 reflected_vp; vec4 eye_time; vec4 right_quality; vec4 up_rt;
    vec4 river; vec4 moon; vec4 settings; vec4 fog_lights; vec4 grid;
};
layout(set=0,binding=1,std430) readonly buffer Waves {vec4 waves[];};
layout(location=0) out vec3 world;
layout(location=1) out vec3 norm;
layout(location=2) out vec2 tex;
layout(location=3) out vec3 colour;
layout(location=4) flat out int kind;
void main() {
    world=position; norm=normal; tex=uv; colour=tint; kind=int(material);
    if(kind==4) {
        ivec2 cell=ivec2(round(uv*(grid.xy-1.)));
        vec4 wave=waves[cell.y*int(grid.x)+cell.x];
        float a=world.x*1.25+world.z*.55-eye_time.w*.8;
        float b=world.x*.7-world.z*1.8+eye_time.w*.65;
        world.y+=wave.x*.35+.025*sin(a)+.012*sin(b);
        norm=normalize(vec3(-wave.y*.25-.03125*cos(a)-.0084*cos(b),1.,-wave.z*.25-.01375*cos(a)+.0216*cos(b)));
        colour=vec3(wave.w);
    }
    gl_Position=vp*vec4(world,1);
}
