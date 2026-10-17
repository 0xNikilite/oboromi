#version 450
layout(location=0) in vec2 uv;
layout(location=1) in vec4 colour;
layout(location=2) flat in vec2 kind_seed;
layout(location=3) in vec3 world;
layout(location=0) out vec4 output_colour;
layout(set=1,binding=0,std140) uniform Frame {
    mat4 vp; mat4 reflected_vp; vec4 eye_time; vec4 right_quality; vec4 up_rt;
    vec4 river; vec4 moon; vec4 settings;
};
struct Light { vec4 position; vec4 colour; };
layout(set=1,binding=1,std430) readonly buffer Lights { Light lights[]; };
layout(set=1,binding=12,std430) readonly buffer Depth { uvec2 background[]; };
float noise(vec2 p) {
    vec2 i=floor(p),f=fract(p);f=f*f*(3.-2.*f);
    vec4 n=fract(sin(vec4(dot(i,vec2(127.1,311.7)),dot(i+vec2(1,0),vec2(127.1,311.7)),dot(i+vec2(0,1),vec2(127.1,311.7)),dot(i+1.,vec2(127.1,311.7))))*43758.5453);
    return mix(mix(n.x,n.y,f.x),mix(n.z,n.w,f.x),f.y);
}
void main(){
    float seed=kind_seed.y,kind=kind_seed.x, a; vec3 c=colour.rgb;
    if(kind<.5){
        float y=uv.y*.5+.5;float distortion=(noise(vec2(y*8.-eye_time.w*2.3,seed))-.5)*y*.45+sin(y*11.-eye_time.w*9.+seed)*.12*y+sin(y*23.-eye_time.w*13.+seed)*.055;
        float width=(1.-y)*.65+.03;
        float body=1.-smoothstep(width*.35,width,abs(uv.x+distortion));
        vec2 flow=vec2((uv.x+distortion)*6.+seed,y*12.-eye_time.w*3.);
        float turbulence=noise(flow)*.5+noise(flow*2.03)*.3+noise(flow*4.07)*.2;
        a=body*smoothstep(0.,.12,y)*(1.-smoothstep(.7,1.,y))*smoothstep(.3,.72,turbulence+(1.-y)*.2);
        float core=pow(max(0.,1.-abs(uv.x+distortion)/max(width,.01)),3.)*(1.-y);
        c=mix(vec3(1.5,.025,.001),vec3(4.,.7,.025),clamp(turbulence+core,0.,1.));
        c=mix(c,vec3(7.,3.,.4),core*turbulence);a*=colour.a;
    }else if(kind<1.5){a=exp(-dot(uv,uv)*5.)*(1.-smoothstep(.6,1.,length(uv)))*colour.a;c*=12.;}
    else if(kind<2.5){a=pow(max(0.,1.-dot(uv,uv)),2.)*colour.a*(.6+.4*sin(uv.x*9.+sin(uv.y*7.)+seed));}
    else if(kind<3.5) { a=pow(max(0.,1.-dot(uv,uv)),3.)*colour.a; }
    else if(kind<4.5) { a=pow(max(0.,1.-dot(uv,uv)),2.)*colour.a; c=vec3(.15,.3,.35)+pow(max(0.,1.-length(uv-vec2(-.2,.25))*3.),8.)*3.; }
    else if(kind<5.5) { a=exp(-pow((length(uv)-.8)*30.,2.))*colour.a; }
    else if(kind<6.5) { a=exp(-pow((length(uv)-.75)*18.,2.))*colour.a;c=vec3(.2,.35,.4); }
    else if(kind<7.5) { a=(1.-smoothstep(.65,.85,abs(uv.x)+abs(uv.y)*.5))*colour.a;c*=.5+.5*cos(uv.y*10.); }
    else if(kind<8.5) { float r=length(uv),angle=atan(uv.y,uv.x);float strand=min(abs(sin(angle*8.))*r,abs(fract(r*8.)-.5)*.12);a=(1.-smoothstep(.002,.012,strand))*(1.-smoothstep(.6,1.,r))*colour.a; }
    else if(kind<9.5) { vec2 q=vec2(abs(uv.x)-.43,uv.y-.25);float eye=1.-smoothstep(.1,.2,abs(q.x)+abs(q.y)*.6);float mouth=(1.-smoothstep(.09,.15,abs(uv.y+.35+.15*cos(uv.x*5.))))*(1.-smoothstep(.6,.8,abs(uv.x)));a=max(eye,mouth); }
    else { a=float(abs(uv.x)<.9 && uv.y>-.35+abs(uv.x)*.4 && uv.y<.32-abs(uv.x)*.25+.12*abs(sin(uv.x*16.))); }
    if(kind>=2. && kind<9.) {
        vec3 illumination=vec3(.1,.13,.16);
        for(int i=0;i<lights.length();i++) {vec3 to=lights[i].position.xyz-world;illumination+=lights[i].colour.rgb*1.4/max(1.,dot(to,to));}
        c*=illumination;
    }
    if(settings.z<.5) {
        ivec2 pixel=ivec2(gl_FragCoord.xy);float depth=unpackHalf2x16(background[pixel.y*int(settings.x)+pixel.x].y).y;
        a*=clamp((depth-distance(world,eye_time.xyz))*(kind==2.?2.:30.),0.,1.);
    }
    if((kind==4. || kind==6.) && settings.z<.5) {
        ivec2 p=clamp(ivec2(gl_FragCoord.xy+uv*3.),ivec2(0),ivec2(settings.xy)-1);
        uvec2 refracted_pixel=background[p.y*int(settings.x)+p.x];
        c+=vec3(unpackHalf2x16(refracted_pixel.x),unpackHalf2x16(refracted_pixel.y).x)*.65;
    }
    if(a<.001)discard;
    output_colour=vec4(c*a,a);
}
