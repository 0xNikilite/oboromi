#version 460
#ifdef RAY_QUERY
#extension GL_EXT_ray_query : require
layout(set=1,binding=14) uniform accelerationStructureEXT scene;
#endif
layout(location=0) in vec3 world;
layout(location=1) in vec3 norm;
layout(location=2) in vec2 tex;
layout(location=3) in vec3 colour;
layout(location=4) flat in int kind;
layout(location=0) out vec4 output_colour;
layout(set=1,binding=0,std140) uniform Frame {
    mat4 vp; mat4 reflected_vp; vec4 eye_time; vec4 right_quality; vec4 up_rt;
    vec4 river; vec4 moon; vec4 settings;
};
struct Light { vec4 position; vec4 colour; };
layout(set=1,binding=1,std430) readonly buffer Lights { Light lights[]; };
layout(set=1,binding=2) uniform sampler2D floor_colour;
layout(set=1,binding=3) uniform sampler2D floor_normal;
layout(set=1,binding=4) uniform sampler2D floor_surface;
layout(set=1,binding=5) uniform sampler2D rock_colour;
layout(set=1,binding=6) uniform sampler2D rock_normal;
layout(set=1,binding=7) uniform sampler2D rock_surface;
layout(set=1,binding=8) uniform sampler2D iron_colour;
layout(set=1,binding=9) uniform sampler2D iron_normal;
layout(set=1,binding=10) uniform sampler2D iron_surface;
layout(set=1,binding=11,std430) readonly buffer Reflection { uvec2 reflection[]; };
layout(set=1,binding=12,std430) readonly buffer Refraction { uvec2 refraction[]; };
struct Vertex { vec4 position; vec4 normal; vec4 tint; };
layout(set=1,binding=13,std430) readonly buffer Geometry { Vertex triangles[]; };
layout(set=1,binding=15,std430) readonly buffer Shadows { uvec2 shadows[]; };
const float PI=3.14159265;
float hash(vec2 p) { return fract(sin(dot(p,vec2(127.1,311.7)))*43758.5453); }
float noise(vec2 p) {
    vec2 i=floor(p),f=fract(p); f=f*f*(3.-2.*f);
    return mix(mix(hash(i),hash(i+vec2(1,0)),f.x),mix(hash(i+vec2(0,1)),hash(i+1.),f.x),f.y);
}
vec3 decode(uvec2 p) { return vec3(unpackHalf2x16(p.x),unpackHalf2x16(p.y).x); }
vec3 screenSample(vec2 uv, bool reflect_scene) {
    ivec2 size=ivec2(settings.xy)/(reflect_scene?2:1); ivec2 p=clamp(ivec2(uv*size),ivec2(0),size-1);
    int at=p.y*size.x+p.x;
    return decode(reflect_scene?reflection[at]:refraction[at]);
}
vec4 surface(vec2 uv,int m) { return m==0?texture(floor_surface,uv):m==2?texture(iron_surface,uv):texture(rock_surface,uv); }
vec3 albedo(vec2 uv,int m) { return pow(m==0?texture(floor_colour,uv).rgb:m==2?texture(iron_colour,uv).rgb:texture(rock_colour,uv).rgb,vec3(2.2)); }
vec3 normalTex(vec2 uv,int m) { return (m==0?texture(floor_normal,uv).xyz:m==2?texture(iron_normal,uv).xyz:texture(rock_normal,uv).xyz)*2.-1.; }
float visibility(vec3 p,vec3 n,vec3 to) {
#ifdef RAY_QUERY
    if(up_rt.w>0.5) {
        rayQueryEXT q;
        rayQueryInitializeEXT(q,scene,gl_RayFlagsTerminateOnFirstHitEXT|gl_RayFlagsOpaqueEXT,255,p+n*.025,.005,normalize(to),max(.006,length(to)-.04));
        while(rayQueryProceedEXT(q)){}
        return rayQueryGetIntersectionTypeEXT(q,true)==gl_RayQueryCommittedIntersectionNoneEXT?1.:0.;
    }
#endif
    return 1.;
}
float shadowMap(vec3 direction,int light,vec3 normal) {
    vec3 a=abs(direction);int face;
    if(a.x>a.y && a.x>a.z)face=direction.x>0.?0:1;
    else if(a.y>a.z)face=direction.y>0.?2:3;
    else face=direction.z>0.?4:5;
    const vec3 rights[6]=vec3[](vec3(0,0,1),vec3(0,0,-1),vec3(1,0,0),vec3(-1,0,0),vec3(-1,0,0),vec3(1,0,0));
    const vec3 ups[6]=vec3[](vec3(0,1,0),vec3(0,1,0),vec3(0,0,1),vec3(0,0,1),vec3(0,1,0),vec3(0,1,0));
    float largest=max(a.x,max(a.y,a.z));vec2 uv=vec2(dot(direction,rights[face]),-dot(direction,ups[face]))/largest*.5+.5;
    ivec2 p=ivec2(uv*256.);float result=0.;
    for(int y=0;y<2;y++)for(int x=0;x<2;x++) {
        ivec2 q=clamp(p+ivec2(x,y),ivec2(0),ivec2(255));
        float depth=unpackHalf2x16(shadows[(light*6+face)*256*256+q.y*256+q.x].x).x;
        result+=float(length(direction)<depth+.025+largest*.008/max(.2,abs(dot(normal,normalize(direction)))));
    }
    return result*.25;
}
float softVisibility(vec3 p,vec3 n,vec3 to) {
    vec3 axis=normalize(cross(normalize(to),abs(normalize(to).y)>.9?vec3(1,0,0):vec3(0,1,0)))*.07;
    vec3 other=cross(normalize(to),axis);
    return (visibility(p,n,to+axis)+visibility(p,n,to-axis)+visibility(p,n,to+other)+visibility(p,n,to-other))*.25;
}
vec3 brdf(vec3 base,vec3 n,vec3 v,vec3 l,float rough,float metal) {
    vec3 h=normalize(v+l); float nl=max(dot(n,l),0.), nv=max(dot(n,v),.001), nh=max(dot(n,h),0.);
    float a2=pow(rough,4.), d=nh*nh*(a2-1.)+1., k=pow(rough+1.,2.)/8.;
    vec3 f0=mix(vec3(.04),base,metal), f=f0+(1.-f0)*pow(1.-max(dot(v,h),0.),5.);
    vec3 spec=f*a2/(PI*d*d*4.*(nl*(1.-k)+k)*(nv*(1.-k)+k)+.00001);
    return ((1.-f)*(1.-metal)*base/PI+spec)*nl;
}
vec3 illuminate(vec3 p,vec3 n,vec3 v,vec3 base,float rough,float metal,bool shadow) {
    vec3 c=base*vec3(.045,.055,.065);
    for(int i=0;i<lights.length();i++) {
        vec3 to=lights[i].position.xyz-p; float d2=dot(to,to);
        if(d2>min(180.,lights[i].colour.r*24.) || dot(n,to)<=0.)continue;
        float flicker=1.+.09*sin(eye_time.w*11.7+float(i)*2.3)+.045*sin(eye_time.w*23.1+float(i));
        float vis=(shadow && settings.z<.5)?(up_rt.w>.5?softVisibility(p,n,to):shadowMap(-to,i,n)):1.;
        c+=brdf(base,n,v,normalize(to),rough,metal)*lights[i].colour.rgb*flicker*vis*5./max(.22,d2);
    }
    return c;
}
vec3 reflectedFlames(vec3 p,vec3 direction,float limit) {
    vec3 c=vec3(0);
    for(int i=0;i<lights.length();i++) {
        float flame=lights[i].position.w;if(flame<=0.)continue;
        vec3 to=lights[i].position.xyz+vec3(0,flame*.25,0)-p;
        float along=dot(to,direction);if(along<0. || along>limit)continue;
        vec3 d=to-direction*along;d.y*=.35;
        float glow=exp(-dot(d,d)/max(.002,flame*flame*.07));
        c+=lights[i].colour.rgb*glow*3.;
    }
    return c;
}
vec3 reflected(vec3 p,vec3 direction,vec3 fallback) {
#ifdef RAY_QUERY
    if(up_rt.w>.5) {
        rayQueryEXT q; rayQueryInitializeEXT(q,scene,gl_RayFlagsOpaqueEXT,255,p,.035,direction,45.);
        while(rayQueryProceedEXT(q)){}
        if(rayQueryGetIntersectionTypeEXT(q,true)!=gl_RayQueryCommittedIntersectionNoneEXT) {
            int i=int(rayQueryGetIntersectionPrimitiveIndexEXT(q,true))*3;
            vec2 b=rayQueryGetIntersectionBarycentricsEXT(q,true); vec3 w=vec3(1.-b.x-b.y,b);
            vec3 pos=triangles[i].position.xyz*w.x+triangles[i+1].position.xyz*w.y+triangles[i+2].position.xyz*w.z;
            vec3 n=normalize(triangles[i].normal.xyz*w.x+triangles[i+1].normal.xyz*w.y+triangles[i+2].normal.xyz*w.z);
            int m=int(triangles[i].position.w); vec2 uv=abs(n.y)>.6?pos.xz*.35:abs(n.x)>.6?pos.zy*.35:pos.xy*.35;
            vec3 reflected_texel=m==0?textureLod(floor_colour,uv,2.).rgb:m==2?textureLod(iron_colour,uv,2.).rgb:textureLod(rock_colour,uv,2.).rgb;
            vec3 base=pow(reflected_texel,vec3(2.2))*triangles[i].tint.rgb;
            return illuminate(pos,n,-direction,base,.55,0.,false)+reflectedFlames(p,direction,rayQueryGetIntersectionTEXT(q,true));
        }
        return vec3(.008,.013,.025)+reflectedFlames(p,direction,45.);
    }
#endif
    return fallback;
}
void main() {
    if(settings.z>1.5){output_colour=vec4(distance(world,eye_time.xyz));return;}
    vec3 n=normalize(norm), v=normalize(eye_time.xyz-world); int m=kind;
    if(settings.z>0.5 && world.y<settings.w+.015)discard;
    vec2 uv=tex; vec3 base; float rough=.6,metal=0.,ao=1.; float wet=0.;
    if(m==4) {
        vec2 flow=world.xz*2.+vec2(-eye_time.w*.2,eye_time.w*.04);
        vec2 ripple=vec2(cos(world.x*6.+world.z*3.-eye_time.w*2.1),cos(world.x*4.-world.z*7.+eye_time.w*1.7));
        n=normalize(n+vec3(ripple.x*.12+(noise(flow*3.)-.5)*.12,0,ripple.y*.1+(noise(flow*3.+9.)-.5)*.12));
        vec2 screen=gl_FragCoord.xy/settings.xy;
        vec4 r=reflected_vp*vec4(world,1); vec2 refl=r.xy/r.w*.5+.5;
        vec3 reflected_colour=reflected(world+n*.025,reflect(-v,n),(up_rt.w>.5?vec3(0):screenSample(refl+n.xz*.025,true)));
        float fresnel=.02+.98*pow(1.-max(dot(n,v),0.),5.);
        ivec2 pixel=clamp(ivec2(gl_FragCoord.xy),ivec2(0),ivec2(settings.xy)-1);
        float bed_distance=unpackHalf2x16(refraction[pixel.y*int(settings.x)+pixel.x].y).y;
        float thickness=clamp(bed_distance-distance(world,eye_time.xyz),.05,3.);
        vec3 transmitted=screenSample(screen+n.xz*.018,false)*exp(-vec3(.55,.22,.16)*thickness)+vec3(.003,.008,.007);
        float edge=min(abs(world.z-river.y),abs(world.z-river.w));
        float foam=(1.-smoothstep(.02,.18,edge))*.55+colour.x*.6;
        foam*=smoothstep(.24,.68,noise(flow*5.));
        vec3 c=mix(transmitted,reflected_colour,max(.24,fresnel))+illuminate(world,n,v,vec3(0),.09,0.,true)+vec3(foam*.12);
        output_colour=vec4(c,distance(world,eye_time.xyz)); return;
    }
    vec3 dp1=dFdx(world),dp2=dFdy(world); vec2 du1=dFdx(tex),du2=dFdy(tex);
    vec3 t=normalize(dp1*du2.y-dp2*du1.y); t=normalize(t-n*dot(n,t));
    vec3 b=normalize(cross(n,t))*sign(du1.x*du2.y-du1.y*du2.x); mat3 basis=mat3(t,b,n);
    if(m!=3) {
        if(m==0) {
            vec3 tv=transpose(basis)*v; float steps=right_quality.w>.5?mix(28.,12.,abs(tv.z)):mix(12.,6.,abs(tv.z));
            vec2 delta=tv.xy/max(abs(tv.z),.15)*.035/steps; float z=0.;
            for(int i=0;i<28;i++) { if(z>=1.-surface(uv,m).a)break; uv-=delta;z+=1./steps; }
        }
        vec4 s=surface(uv,m); rough=max(.36,s.g);metal=s.b;ao=s.r;
        base=albedo(uv,m)*colour;
        float moss=(1.-s.r)*smoothstep(.35,.65,noise(world.xz*.65+world.y));
        base=mix(base,vec3(.025,.06,.012),moss*.6);
        wet=m==0?smoothstep(.43,.64,noise(world.xz*.43)):(m==1?.08:0.);
        base*=1.-wet*.38; rough=mix(rough,.085,wet);
        vec3 nt=normalTex(uv,m);nt.xy*=1.-wet*.65;n=normalize(basis*nt);
    } else { base=colour*mix(vec3(.24),albedo(uv,1),.2); rough=clamp(surface(uv,1).g,.4,.85); }
    vec3 c=illuminate(world,n,v,base,max(.065,rough),metal,true)*ao;
    if(m==0 && wet>.05 && settings.z<.5) {
        vec4 r=reflected_vp*vec4(world,1);vec2 refl=r.xy/r.w*.5+.5;
        float f=.035+.965*pow(1.-max(dot(n,v),0.),5.);
        c=mix(c,reflected(world+n*.03,reflect(-v,n),(up_rt.w>.5?vec3(0):screenSample(refl+n.xz*.012,true))),wet*f*.8);
    }
#ifdef RAY_QUERY
    if(up_rt.w>.5 && right_quality.w>.5 && settings.z<.5) {
        vec3 axis=normalize(cross(n,abs(n.y)>.8?vec3(1,0,0):vec3(0,1,0)));
        vec3 dir=normalize(n+axis*.65); c*=.6+.4*visibility(world,n,dir*.85);
    }
#endif
    if(world.y<-.8) c+=base*pow(.5+.5*sin(world.x*9.+sin(world.z*7.+eye_time.w)*2.+eye_time.w*1.8),12.)*.22;
    float distance_to_eye=length(eye_time.xyz-world);
    float fog=1.-exp(-distance_to_eye*.024*exp(-max(world.y,0.)*.18));
    c=mix(c,vec3(.035,.05,.067),fog);
    output_colour=vec4(c,distance(world,eye_time.xyz));
}
