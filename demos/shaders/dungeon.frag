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
    vec4 river; vec4 moon; vec4 settings; vec4 fog_lights; vec4 grid;
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
layout(set=1,binding=16,std430) readonly buffer Rooms { vec4 rooms[]; };
const float PI=3.14159265;
float hash(vec2 p) { return fract(sin(dot(p,vec2(127.1,311.7)))*43758.5453); }
float noise(vec2 p) {
    vec2 i=floor(p),f=fract(p); f=f*f*(3.-2.*f);
    return mix(mix(hash(i),hash(i+vec2(1,0)),f.x),mix(hash(i+vec2(0,1)),hash(i+1.),f.x),f.y);
}
vec3 decode(uvec2 p) { return vec3(unpackHalf2x16(p.x),unpackHalf2x16(p.y).x); }
vec3 screenSample(vec2 uv, bool reflect_scene) {
    ivec2 size=ivec2(settings.xy)/(reflect_scene?(right_quality.w>.5?2:4):1);
    ivec2 p=clamp(ivec2(uv*size),ivec2(0),size-1);int at=p.y*size.x+p.x;
    return decode(reflect_scene?reflection[at]:refraction[at]);
}
vec3 reflectionLinear(vec2 uv) {
    ivec2 size=ivec2(settings.xy)/(right_quality.w>.5?2:4);
    vec2 pixel=uv*vec2(size)-.5, f=fract(pixel);ivec2 base=ivec2(floor(pixel));
    vec3 samples[4];
    for(int i=0;i<4;i++) {
        ivec2 p=clamp(base+ivec2(i%2,i/2),ivec2(0),size-1);int at=p.y*size.x+p.x;
        samples[i]=decode(reflection[at]);
    }
    return mix(mix(samples[0],samples[1],f.x),mix(samples[2],samples[3],f.x),f.y);
}
vec3 roughReflection(vec2 uv,float rough) {
    vec2 radius=vec2(rough*12.)/settings.xy;
    return (reflectionLinear(uv)*2.+reflectionLinear(uv+radius)+reflectionLinear(uv-radius)
        +reflectionLinear(uv+radius*vec2(1,-1))+reflectionLinear(uv+radius*vec2(-1,1)))/6.;
}

vec4 surface(vec2 uv,int m) { return m==0?texture(floor_surface,uv):m==2?texture(iron_surface,uv):texture(rock_surface,uv); }
vec3 albedo(vec2 uv,int m) { return pow(m==0?texture(floor_colour,uv).rgb:m==2?texture(iron_colour,uv).rgb:texture(rock_colour,uv).rgb,vec3(2.2)); }
vec3 normalTex(vec2 uv,int m) { return (m==0?texture(floor_normal,uv).xyz:m==2?texture(iron_normal,uv).xyz:texture(rock_normal,uv).xyz)*2.-1.; }
float fbmNoise(vec2 p) { return noise(p)*.57+noise(p*2.03+17.)*.29+noise(p*4.09+41.)*.14; }
float riverDistance(vec2 p) { vec2 d=max(max(river.xy-p,p-river.zw),0.);return length(d); }
float sootAt(vec3 p) {
    float soot=0.;
    for(int i=0;i<lights.length();i++) {
        float flame=lights[i].position.w,rise=p.y-lights[i].position.y;if(flame<=0. || rise<=0.)continue;
        float spread=.16+rise*(flame>1.? .42:.16),fade=1.-smoothstep(flame>1.?7.:3.,flame>1.?9.:4.2,rise);
        soot=max(soot,exp(-dot(p.xz-lights[i].position.xz,p.xz-lights[i].position.xz)/(spread*spread))*fade);
    }
    return soot;
}
vec4 stoneMaterial(vec3 texel,vec3 p,vec3 n,int m,vec2 uv) {
    float zone=fbmNoise(p.xz*.13+vec2(p.y*.07,-p.y*.05));
    if(m==1) {
        float along=abs(n.x)>abs(n.z)?p.z:p.x,row=floor(p.y/.42);
        vec2 joint=abs((abs(n.y)>.7?fract(p.xz/vec2(.88,.52)):fract(vec2(along/.88+mod(row,2.)*.5,p.y/.42)))-.5);
        float mortar=smoothstep(.455,.49,max(joint.x,joint.y)),wear=1.-smoothstep(0.,.58,p.y);
        vec3 dressed=mix(vec3(.052,.06,.066),texel*vec3(.4,.43,.47),.22);
        dressed=mix(dressed,vec3(.018,.021,.022),mortar);dressed*=1.-wear*.24;
        return vec4(dressed,mix(.82,.58,wear));
    }
    if(m==8) {
        vec3 pebble=mix(vec3(.25,.27,.27),vec3(.43,.34,.23),smoothstep(.32,.75,zone));
        pebble=mix(pebble,texel*vec3(1.1,.98,.8),.32);
        float submerged=1.-smoothstep(-.75,-.3,p.y);
        return vec4(pebble*(1.-submerged*.42),mix(.76,.32,submerged));
    }
    float along=abs(n.x)>abs(n.z)?p.z:p.x;
    if(m==7) {
        float amber=smoothstep(.5,.76,fbmNoise(vec2(along*.28+p.y*.035,p.y*.13-p.x*.025)));
        float tip=smoothstep(.62,.98,uv.y),crust=smoothstep(.72,.84,fbmNoise(p.xz*.19+vec2(p.y*.11)))*(1.-tip*.7);
        vec3 calcite=mix(vec3(.68,.63,.52),vec3(.83,.43,.13),amber*.72);
        calcite=mix(calcite,texel*vec3(1.35,1.18,.88),.18);
        float soot=max(crust*.7,sootAt(p));calcite=mix(calcite,vec3(.018,.013,.009),soot*.84);
        return vec4(calcite,mix(mix(.28,.045,tip),.94,soot));
    }
    float vertical=1.-smoothstep(.48,.88,abs(n.y));
    float flow=vertical*smoothstep(.54,.72,fbmNoise(vec2(along*.2,p.y*.045+p.x*.018)));
    float iron=smoothstep(.66,.82,fbmNoise(vec2(p.y*.31+zone,along*.065)))*(1.-flow*.45);
    float crack=1.-smoothstep(.025,.08,abs(fract(along*.19+noise(vec2(p.y*.17,dot(p.xz,vec2(.07,.11))))*.72)-.5));
    float manganese=crack*smoothstep(.56,.72,fbmNoise(p.xz*.14+vec2(p.y*.05,-p.y*.08)));
    vec3 limestone=mix(vec3(.37,.39,.36),vec3(.56,.48,.34),smoothstep(.28,.78,zone));
    limestone=mix(limestone,texel*vec3(1.4,1.28,1.02),.24);
    limestone=mix(limestone,mix(vec3(.72,.68,.56),vec3(.76,.42,.15),iron),flow*.78);
    limestone=mix(limestone,vec3(.46,.16,.045),iron*.48);
    float riverWet=1.-smoothstep(.08,1.,length(vec2(riverDistance(p.xz),max(p.y+.7,0.))));
    float bio=smoothstep(-.8,-.66,p.y)*(1.-smoothstep(-.25,-.12,p.y))*(1.-smoothstep(.04,.55,riverDistance(p.xz)));
    float soot=max(manganese*.82,sootAt(p));
    limestone*=1.-riverWet*.42;limestone=mix(limestone,vec3(.025,.033,.018),bio*.78);limestone=mix(limestone,vec3(.014,.01,.007),soot*.88);
    return vec4(limestone,mix(mix(.76,.28,riverWet),.95,max(bio,soot)));
}
float visibility(vec3 p,vec3 n,vec3 to) {
#ifdef RAY_QUERY
    if(up_rt.w>0.5) {
        rayQueryEXT q;
        rayQueryInitializeEXT(q,scene,gl_RayFlagsTerminateOnFirstHitEXT|gl_RayFlagsOpaqueEXT,255,p+n*.012,.003,normalize(to),max(.004,length(to)-.02));
        while(rayQueryProceedEXT(q)){}
        return rayQueryGetIntersectionTypeEXT(q,true)==gl_RayQueryCommittedIntersectionNoneEXT?1.:0.;
    }
#endif
    return 1.;
}
float shadowMap(vec3 direction,int light,vec3 normal) {
    int slot=int(lights[light].colour.w)-1;
    if(slot<0)return 1.;
    int size=right_quality.w>.5?256:128;
    vec3 a=abs(direction);int face;
    if(a.x>a.y && a.x>a.z)face=direction.x>0.?0:1;
    else if(a.y>a.z)face=direction.y>0.?2:3;
    else face=direction.z>0.?4:5;
    const vec3 rights[6]=vec3[](vec3(0,0,1),vec3(0,0,-1),vec3(1,0,0),vec3(-1,0,0),vec3(-1,0,0),vec3(1,0,0));
    const vec3 ups[6]=vec3[](vec3(0,1,0),vec3(0,1,0),vec3(0,0,1),vec3(0,0,1),vec3(0,1,0),vec3(0,1,0));
    float largest=max(a.x,max(a.y,a.z));vec2 uv=vec2(dot(direction,rights[face]),-dot(direction,ups[face]))/largest*.5+.5;
    ivec2 p=ivec2(uv*float(size));float result=0.;
    for(int y=0;y<2;y++)for(int x=0;x<2;x++) {
        ivec2 q=clamp(p+ivec2(x,y),ivec2(0),ivec2(size-1));
        float depth=unpackHalf2x16(shadows[(slot*6+face)*size*size+q.y*size+q.x].x).x;
        result+=float(length(direction)<depth+.025+largest*.008/max(.2,abs(dot(normal,normalize(direction)))));
    }
    return result*.25;
}
float softVisibility(vec3 p,vec3 n,vec3 to,float radius) {
    vec3 direction=normalize(to);
    vec3 axis=normalize(cross(direction,abs(direction.y)>.9?vec3(1,0,0):vec3(0,1,0)))*radius;
    vec3 other=cross(direction,axis);
    const vec2 disk[5]=vec2[](vec2(0),vec2(.92,.18),vec2(-.24,.95),vec2(-.88,-.32),vec2(.31,-.9));
    float open=0.;
    for(int i=0;i<5;i++)open+=visibility(p,n,to+axis*disk[i].x+other*disk[i].y);
    return open/5.;
}
float ambientVisibility(vec3 p,vec3 n) {
#ifdef RAY_QUERY
    vec3 tangent=normalize(cross(n,abs(n.y)>.8?vec3(1,0,0):vec3(0,1,0))),bitangent=cross(n,tangent);
    const vec3 rays[3]=vec3[](vec3(.18,.12,.98),vec3(-.57,.21,.79),vec3(.42,-.61,.67));
    float open=0.;
    for(int i=0;i<3;i++){vec3 d=normalize(tangent*rays[i].x+bitangent*rays[i].y+n*rays[i].z);open+=visibility(p,n,d*(i==0?.38:1.15));}
    return open/3.;
#else
    return 1.;
#endif
}
vec3 brdf(vec3 base,vec3 n,vec3 v,vec3 l,float rough,float metal) {
    vec3 h=normalize(v+l); float nl=max(dot(n,l),0.), nv=max(dot(n,v),.001), nh=max(dot(n,h),0.);
    float a2=pow(rough,4.), d=nh*nh*(a2-1.)+1., k=pow(rough+1.,2.)/8.;
    vec3 f0=mix(vec3(.04),base,metal), f=f0+(1.-f0)*pow(1.-max(dot(v,h),0.),5.);
    vec3 spec=f*a2/(PI*d*d*4.*(nl*(1.-k)+k)*(nv*(1.-k)+k)+.00001);
    return ((1.-f)*(1.-metal)*base/PI+spec)*nl;
}
vec3 illuminate(vec3 p,vec3 n,vec3 v,vec3 base,float rough,float metal,bool shadow) {
    vec3 c=up_rt.w>.5 && right_quality.w>.5 && settings.z<.5?vec3(0):base*vec3(.045,.055,.065);
    for(int j=0;j<(right_quality.w>.5?lights.length():6);j++) {
        int i=right_quality.w>.5?j:int(j<4?fog_lights[j]:grid[j-2]);
        vec3 to=lights[i].position.xyz-p; float d2=dot(to,to);
        if(d2>min(180.,lights[i].colour.r*24.) || dot(n,to)<=0.)continue;
        bool casts=lights[i].position.w>0. && (right_quality.w>.5 || i==int(fog_lights.x) || i==int(fog_lights.y));
        float vis=(shadow && casts && settings.z<.5)?(up_rt.w>.5?softVisibility(p,n,to,clamp(lights[i].position.w*.32,.12,.5)):shadowMap(-to,i,n)):1.;
        c+=brdf(base,n,v,normalize(to),rough,metal)*lights[i].colour.rgb*vis*5./max(.22,d2);
    }
    return c;
}
vec3 calciteTransmission(vec3 p,vec3 n,vec3 v,float thin) {
    vec3 c=vec3(0);
    for(int i=0;i<lights.length();i++) {
        vec3 to=lights[i].position.xyz-p;float d2=dot(to,to),back=max(dot(-n,normalize(to)),0.);
        c+=lights[i].colour.rgb*back*(.012+.035*thin)*(.3+.7*pow(1.-abs(dot(n,v)),2.))/max(.5,d2);
    }
    return c;
}
#ifdef RAY_QUERY
vec3 diffuseBounce(vec3 p,vec3 n,vec3 base) {
    vec3 tangent=normalize(cross(n,abs(n.y)>.8?vec3(1,0,0):vec3(0,1,0))),bitangent=cross(n,tangent),c=vec3(0);
    const vec3 rays[2]=vec3[](vec3(.53,.21,.82),vec3(-.37,.49,.79));
    for(int ray=0;ray<2;ray++) {
        vec3 direction=normalize(tangent*rays[ray].x+bitangent*rays[ray].y+n*rays[ray].z);
        rayQueryEXT q;rayQueryInitializeEXT(q,scene,gl_RayFlagsOpaqueEXT,255,p+n*.02,.025,direction,12.);
        while(rayQueryProceedEXT(q)){}
        if(rayQueryGetIntersectionTypeEXT(q,true)==gl_RayQueryCommittedIntersectionNoneEXT)continue;
        int i=int(rayQueryGetIntersectionPrimitiveIndexEXT(q,true))*3;vec2 b=rayQueryGetIntersectionBarycentricsEXT(q,true);vec3 w=vec3(1.-b.x-b.y,b);
        vec3 hit=triangles[i].position.xyz*w.x+triangles[i+1].position.xyz*w.y+triangles[i+2].position.xyz*w.z;
        vec3 hitNormal=normalize(triangles[i].normal.xyz*w.x+triangles[i+1].normal.xyz*w.y+triangles[i+2].normal.xyz*w.z);
        int m=int(triangles[i].position.w);vec2 uv=abs(hitNormal.y)>.6?hit.xz*.35:abs(hitNormal.x)>.6?hit.zy*.35:hit.xy*.35;
        vec3 hitBase=albedo(uv,m)*triangles[i].tint.rgb;if(m==1 || m>=6)hitBase=stoneMaterial(hitBase,hit,hitNormal,m,uv).rgb;
        c+=illuminate(hit,hitNormal,-direction,hitBase,.72,0.,false)*max(dot(n,direction),0.);
    }
    return base*c*.48;
}
#endif
vec3 reflectedFlames(vec3 p,vec3 direction,float limit) {
    vec3 c=vec3(0);
    for(int i=0;i<lights.length();i++) {
        float flame=lights[i].position.w;if(flame<=0.)continue;
        vec3 to=lights[i].position.xyz+vec3(0,flame*.25,0)-p;
        float along=dot(to,direction);if(along<0. || along>limit)continue;
        vec3 d=to-direction*along;d.y*=.5;
        float glow=exp(-dot(d,d)/max(.002,flame*flame*.065));
        c+=lights[i].colour.rgb*glow*4.;
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
            if(m==1 || m>=6)base=stoneMaterial(base,pos,n,m,uv).rgb;
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
        n=normalize(n+vec3(ripple.x*.018+(noise(flow*3.)-.5)*.012,0,ripple.y*.015+(noise(flow*3.+9.)-.5)*.012));
        vec2 screen=gl_FragCoord.xy/settings.xy;
        vec4 r=reflected_vp*vec4(world,1); vec2 refl=r.xy/r.w*.5+.5;
        vec3 reflected_colour=reflected(world+n*.025,reflect(-v,n),(up_rt.w>.5?vec3(0):roughReflection(refl+n.xz*.012,.24)));
        if(up_rt.w>.5) {
            vec3 direction=reflect(-v,n), tangent=normalize(cross(direction,abs(direction.y)>.9?vec3(1,0,0):vec3(0,1,0)))*.009;
            reflected_colour=(reflected_colour*2.+reflected(world+n*.025,normalize(direction+tangent),vec3(0))+reflected(world+n*.025,normalize(direction-tangent),vec3(0)))*.25;
        }
        float fresnel=.02+.98*pow(1.-max(dot(n,v),0.),5.);
        ivec2 pixel=clamp(ivec2(gl_FragCoord.xy),ivec2(0),ivec2(settings.xy)-1);
        float bed_distance=unpackHalf2x16(refraction[pixel.y*int(settings.x)+pixel.x].y).y;
        float thickness=clamp(bed_distance-distance(world,eye_time.xyz),.05,3.);
        vec3 transmitted=screenSample(screen+n.xz*.018,false)*exp(-vec3(.18,.085,.055)*thickness)+vec3(.008,.022,.026)*(1.-exp(-thickness));
        float edge=min(tex.y,1.-tex.y)*(river.w-river.y);
        float bank=(1.-smoothstep(.015,.16,edge));
        float crest=smoothstep(.05,.14,world.y+.7)*smoothstep(.28,.55,length(norm.xz))*smoothstep(.45,.75,noise(flow*2.));
        float foam=max(bank*.45, max(colour.x,crest)*.65);
        foam*=smoothstep(.45,.75,noise(vec2(world.x*3.-eye_time.w*.7,world.z*14.)));
        vec3 c=mix(transmitted,reflected_colour,fresnel)+illuminate(world,n,v,vec3(0),.095,0.,true)+foam*vec3(.28,.34,.32);
        if((uint(moon.w)&16u)!=0u)c=n*.5+.5;
        if((uint(moon.w)&32u)!=0u)c=reflected_colour;
        if((uint(moon.w)&64u)!=0u)c=transmitted;
        output_colour=vec4(c,distance(world,eye_time.xyz)); return;
    }
    vec3 dp1=dFdx(world),dp2=dFdy(world); vec2 du1=dFdx(tex),du2=dFdy(tex);
    vec3 t=normalize(dp1*du2.y-dp2*du1.y); t=normalize(t-n*dot(n,t));
    vec3 b=normalize(cross(n,t))*sign(du1.x*du2.y-du1.y*du2.x); mat3 basis=mat3(t,b,n);
    if(m!=3) {
        bool projected=m==1 || m==6;
        if(projected && right_quality.w<.5)uv=abs(n.x)>max(abs(n.y),abs(n.z))?world.zy*.35:abs(n.y)>abs(n.z)?world.xz*.35:world.xy*.35;
        if(m==0) {
            vec3 tv=transpose(basis)*v; float steps=right_quality.w>.5?mix(28.,12.,abs(tv.z)):mix(12.,6.,abs(tv.z));
            vec2 delta=tv.xy/max(abs(tv.z),.15)*.035/steps; float z=0.;
            for(int i=0;i<28;i++) { if(z>=1.-surface(uv,m).a)break; uv-=delta;z+=1./steps; }
        }
        vec4 s=surface(uv,m); rough=max(.36,s.g);metal=s.b;ao=s.r;
        base=albedo(uv,m)*colour;
        if(projected && right_quality.w>.5){
            vec3 w=pow(abs(n),vec3(5.));w/=w.x+w.y+w.z;
            base=(albedo(world.zy*.35,1)*w.x+albedo(world.xz*.35,1)*w.y+albedo(world.xy*.35,1)*w.z)*colour;
            s=surface(world.zy*.35,1)*w.x+surface(world.xz*.35,1)*w.y+surface(world.xy*.35,1)*w.z;
            rough=max(.48,s.g);ao=s.r;
        }
        float moss=float(m==1)*(1.-s.r)*smoothstep(.35,.65,noise(world.xz*.65+world.y));
        base=mix(base,vec3(.025,.06,.012),moss*.35);
        wet=m==1?.14:0.;
        if(m==0){
            float macro=noise(world.xz*.19)*.65+noise(world.xz*.77+13.)*.35;
            float wall=10.;
            for(int i=0;i<rooms.length();i++) {
                vec4 r=rooms[i];vec2 d=min(world.xz-r.xy,r.zw-world.xz);
                if(min(d.x,d.y)>=-.02)wall=min(wall,min(d.x,d.y));
            }
            float grime=(1.-smoothstep(0.,.65,wall))*smoothstep(.2,.7,noise(world.xz*3.1));
            base*=mix(vec3(.52,.59,.43),vec3(1.12,.94,.76),macro);
            base=mix(base,vec3(.018,.035,.009),grime*(1.-s.r*.6)*.75);
            rough=clamp(rough+.18+(macro-.5)*.22,.62,.96);
            wet=smoothstep(.53,.64,noise(world.xz*.43)+noise(world.xz*1.7)*.13-grime*.22);
        }else if(m==1 || m>=6){vec4 stone=stoneMaterial(base,world,n,m,uv);base=stone.rgb;rough=stone.a;}
        base*=1.-wet*.38; rough=mix(rough,.055,wet);
        vec3 nt=normalTex(uv,m);nt.xy*=m==1?.35:1.-wet*.93;
        if(projected && right_quality.w<.5){
            n=normalize(abs(n.x)>max(abs(n.y),abs(n.z))?vec3(nt.z*sign(n.x),nt.y,nt.x):abs(n.y)>abs(n.z)?vec3(nt.x,nt.z*sign(n.y),nt.y):vec3(nt.x,nt.y,nt.z*sign(n.z)));
        }else if(projected){
            vec3 w=pow(abs(n),vec3(5.));w/=w.x+w.y+w.z;
            vec3 nx=normalTex(world.zy*.35,1),ny=normalTex(world.xz*.35,1),nz=normalTex(world.xy*.35,1);
            n=normalize(vec3(nx.z*sign(n.x),nx.y,nx.x)*w.x+vec3(ny.x,ny.z*sign(n.y),ny.y)*w.y+vec3(nz.x,nz.y,nz.z*sign(n.z))*w.z);
        }else n=normalize(basis*nt);
    } else { base=colour*mix(vec3(.24),albedo(uv,1),.2); rough=clamp(surface(uv,1).g,.4,.85); }
    if(m==5){base=colour;rough=.2;metal=1.;}
    vec3 c=illuminate(world,n,v,base,max(.065,rough),metal,true)*ao;
    if(m==7)c+=base*calciteTransmission(world,n,v,smoothstep(.62,.98,tex.y));
#ifdef RAY_QUERY
    if(up_rt.w>.5 && m==7 && rough<.45 && settings.z<.5) {
        float f=.06+.94*pow(1.-max(dot(n,v),0.),5.);
        c=mix(c,reflected(world+n*.03,reflect(-v,n),vec3(0)),f*(1.-smoothstep(.08,.45,rough))*.58);
    }
#endif
    if(m==0 && wet>.05 && settings.z<.5) {
        vec4 r=reflected_vp*vec4(world,1);vec2 refl=r.xy/r.w*.5+.5;
        float f=.035+.965*pow(1.-max(dot(n,v),0.),5.);
        c=mix(c,reflected(world+n*.03,reflect(-v,n),(up_rt.w>.5?vec3(0):screenSample(refl+n.xz*.012,true))),wet*f*.8);
    }
#ifdef RAY_QUERY
    bool traced=up_rt.w>.5 && right_quality.w>.5 && settings.z<.5;
    if(traced)c+=diffuseBounce(world,n,base)*ambientVisibility(world,n);
#else
    const bool traced=false;
#endif
    if(world.y<-.8 && !traced)c+=base*.4+base*pow(.5+.5*sin(world.x*9.+sin(world.z*7.+eye_time.w)*2.+eye_time.w*1.8),12.)*.22;
    output_colour=vec4(c,distance(world,eye_time.xyz));
}
