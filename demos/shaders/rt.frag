// Cornell-room PBR ray queries. light_dir and ambient are the key and cool light positions, with radius in w.
#version 460
#extension GL_EXT_ray_query : require

layout(location=0) in vec3 in_world; layout(location=1) in vec3 in_normal;
layout(location=2) in vec4 in_tangent; layout(location=3) in vec2 in_uv;
layout(std140,set=0,binding=1) uniform Fragment { vec4 light_dir; vec4 light_colour; vec4 ambient; vec4 camera_pos; vec4 base_colour_factor; vec4 emissive_factor; vec4 factors; vec4 sky; };
layout(set=0,binding=2) uniform sampler2D base_colour_map; layout(set=0,binding=3) uniform sampler2D normal_map;
layout(set=0,binding=4) uniform sampler2D metallic_roughness_map; layout(set=0,binding=5) uniform sampler2D occlusion_map;
layout(set=0,binding=6) uniform sampler2D emissive_map; layout(set=0,binding=7) uniform accelerationStructureEXT scene;
layout(location=0) out vec4 out_colour;
const float PI=3.14159265;
const int RAYS=8,GLOSSY=4;

vec3 srgb_to_linear(vec3 c){return pow(c,vec3(2.2));}
vec3 aces(vec3 x){return clamp(x*(2.51*x+0.03)/(x*(2.43*x+0.59)+0.14),0.0,1.0);}
float hash(vec2 p){return fract(sin(dot(p,vec2(127.1,311.7)))*43758.5453);}

bool trace(vec3 origin,vec3 dir,float t_max,out float distance,out uint instance){
 rayQueryEXT q; rayQueryInitializeEXT(q,scene,gl_RayFlagsOpaqueEXT,0xff,origin,sky.w,dir,t_max);
 while(rayQueryProceedEXT(q)){} if(rayQueryGetIntersectionTypeEXT(q,true)==gl_RayQueryCommittedIntersectionNoneEXT)return false;
 distance=rayQueryGetIntersectionTEXT(q,true); instance=rayQueryGetIntersectionInstanceIdEXT(q,true); return true;
}
vec3 room_normal(vec3 p,uint instance){
 if(instance==0u)return normalize(p-vec3(0.0,-0.05,0.0));
 vec3 chrome=p-vec3(-1.65,-0.35,-0.55),glass=p-vec3(1.65,-0.35,-0.45);
 if(length(chrome)<0.72)return normalize(chrome); if(length(glass)<0.72)return normalize(glass);
 vec3 d=vec3(abs(abs(p.x)-3.0),abs(p.y+1.0),abs(p.z+3.0));
 if(d.x<d.y&&d.x<d.z)return vec3(-sign(p.x),0,0); if(d.y<d.z)return vec3(0,p.y<0?1:-1,0); return vec3(0,0,1);
}
vec3 room_colour(vec3 p,uint instance){
 if(instance==0u)return mix(vec3(0.035),vec3(0.32,0.35,0.38),smoothstep(-0.2,0.8,p.y));
 if(length(p-vec3(-1.65,-0.35,-0.55))<0.72)return vec3(0.75);
 if(length(p-vec3(1.65,-0.35,-0.45))<0.72)return vec3(0.25,0.55,0.65);
 if(abs(p.y+1.0)<0.05)return vec3(0.025,0.03,0.04); if(p.x< -2.9)return vec3(0.62,0.015,0.02);
 if(p.x>2.9)return vec3(0.015,0.42,0.07); return vec3(0.72,0.64,0.48);
}
vec3 hit_colour(vec3 origin,vec3 dir){
 float distance; uint instance; if(!trace(origin,dir,20.0,distance,instance))return sky.rgb;
 vec3 p=origin+dir*distance,n=room_normal(p,instance),key=normalize(light_dir.xyz-p),cool=normalize(ambient.xyz-p);
 return room_colour(p,instance)*(0.12+light_colour.rgb*max(dot(n,key),0.0)*0.85+vec3(0.18,0.35,0.7)*max(dot(n,cool),0.0)*0.22);
}
vec3 tangent_ray(vec3 axis,float spread,int index,float seed){
 vec3 t=normalize(cross(abs(axis.x)<0.8?vec3(1,0,0):vec3(0,1,0),axis)),b=cross(axis,t);
 float u=(float(index)+hash(gl_FragCoord.xy+seed))/float(RAYS),phi=6.2831853*fract(float(index)*0.6180339+seed);
 return normalize(axis+spread*sqrt(u)*(cos(phi)*t+sin(phi)*b));
}
vec3 bounce_light(vec3 p,uint instance,bool key){
 vec4 light=key?light_dir:ambient; vec3 n=room_normal(p,instance),l=light.xyz-p; float distance=length(l),hit; uint id; l/=distance;
 return trace(p+n*sky.w,l,distance-light.w,hit,id)?vec3(0):room_colour(p,instance)*(key?light_colour.rgb/PI:vec3(0.18,0.35,0.8)*0.45)*max(dot(n,l),0.0);
}
float visibility(vec3 p,vec3 n,vec4 light,float seed){
 vec3 centre=light.xyz-p; float distance=length(centre),open=0.0; vec3 axis=centre/distance;
 for(int i=0;i<RAYS;i++){vec3 ray=tangent_ray(axis,light.w/distance,i,seed);float hit;uint instance;open+=trace(p+n*sky.w,ray,distance-light.w,hit,instance)?0.0:1.0;}
 return open/float(RAYS);
}
vec3 hemisphere(vec3 n,int index,float seed){
 float u=(float(index)+hash(gl_FragCoord.yx+seed))/float(RAYS),phi=6.2831853*fract(float(index)*0.6180339+seed);
 vec3 t=normalize(cross(abs(n.x)<0.8?vec3(1,0,0):vec3(0,1,0),n)); return normalize(t*cos(phi)*sqrt(u)+cross(n,t)*sin(phi)*sqrt(u)+n*sqrt(1.0-u));
}
vec3 reflected(vec3 p,vec3 ray,float roughness){vec3 c=vec3(0);for(int i=0;i<GLOSSY;i++)c+=hit_colour(p,tangent_ray(ray,roughness,i,4.7));return c/float(GLOSSY);}
vec3 glass(vec3 p,vec3 n,vec3 view){
 const vec3 centre=vec3(1.65,-0.35,-0.45); vec3 incident=-view;
 vec3 reflection=hit_colour(p+n*sky.w,reflect(incident,n)),inside=refract(incident,n,1.0/1.5);
 float thickness=max(0.0,-2.0*dot(p-centre,inside)); vec3 exit_point=p+inside*thickness;
 vec3 exit_ray=refract(inside,-normalize(exit_point-centre),1.5);
 vec3 transmission=hit_colour(exit_point+exit_ray*sky.w,exit_ray)*exp(-vec3(0.08,0.025,0.015)*thickness);
 float fresnel=0.04+0.96*pow(1.0-max(dot(view,n),0.0),5.0); return mix(transmission,reflection,fresnel);
}
void main(){
 vec4 base=texture(base_colour_map,in_uv); vec3 albedo=srgb_to_linear(base.rgb)*base_colour_factor.rgb;
 vec2 mr=texture(metallic_roughness_map,in_uv).bg*factors.xy; float metallic=mr.x,roughness=clamp(mr.y,0.04,1.0);
 vec3 tn=texture(normal_map,in_uv).xyz*2.0-1.0;tn.xy*=factors.z;vec3 geometric=normalize(in_normal);
 vec3 t=normalize(in_tangent.xyz-geometric*dot(geometric,in_tangent.xyz));vec3 n=normalize(mat3(t,cross(geometric,t)*in_tangent.w,geometric)*tn);
 vec3 v=normalize(camera_pos.xyz-in_world),key=normalize(light_dir.xyz-in_world),cool=normalize(ambient.xyz-in_world),h=normalize(v+key);
 float nl=max(dot(n,key),0.0),nv=max(dot(n,v),1e-4),nh=max(dot(n,h),0.0),a2=pow(roughness,4.0),d=nh*nh*(a2-1.0)+1.0,k=pow(roughness+1.0,2.0)/8.0;
 vec3 f0=mix(vec3(0.04),albedo,metallic),fresnel=f0+(1.0-f0)*pow(1.0-max(dot(v,h),0.0),5.0);
 vec3 direct=((1.0-fresnel)*(1.0-metallic)*albedo/PI+fresnel*a2/(PI*d*d)/((nl*(1.0-k)+k)*(nv*(1.0-k)+k)*4.0))*light_colour.rgb*nl*(nl>0?visibility(in_world,geometric,light_dir,1.3):0.0);
 direct+=albedo*vec3(0.18,0.35,0.8)*max(dot(n,cool),0.0)*visibility(in_world,geometric,ambient,8.9)*0.45;
 vec3 origin=in_world+geometric*sky.w,indirect=vec3(0);
 bool floor=in_world.y< -0.95&&geometric.y>0.8,chrome=length(in_world-vec3(-1.65,-0.35,-0.55))<0.72,glass_sphere=length(in_world-vec3(1.65,-0.35,-0.45))<0.72;
 if(floor)indirect+=reflected(origin,reflect(-v,n),0.15)*0.72;else if(chrome)indirect=hit_colour(origin,reflect(-v,n));else if(glass_sphere)indirect=glass(in_world,n,v);else if(metallic>0.45)indirect+=reflected(origin,reflect(-v,n),roughness*0.35)*fresnel;
 if(!chrome&&!glass_sphere){vec3 bounce=vec3(0);float ao=0.0;
  // Each direction is traced once per light: shading both lights in one iteration spills registers on SM86.
  for(int i=0;i<RAYS*2;i++){vec3 ray=hemisphere(geometric,i/2,12.1);float distance;uint instance;bool hit=trace(origin,ray,20.0,distance,instance);ao+=hit&&distance<1.0?0.0:1.0;if(hit)bounce+=bounce_light(origin+ray*distance,instance,i%2==0);}
  indirect+=albedo*(vec3(0.045)*ao/2.0+bounce)/float(RAYS);}
 vec3 emissive=srgb_to_linear(texture(emissive_map,in_uv).rgb)*emissive_factor.rgb;
 out_colour=vec4(pow(aces((direct+indirect+emissive)*0.72),vec3(1.0/2.2)),1.0);
}
