// Draws: one full-screen triangle (3 vertices, no vertex streams) covering the target, uv (0,0) at the top-left.
// Inputs: none; corners come from gl_VertexIndex (a[0x2fc]).
// Uniforms: none.
#version 450

layout(location = 0) out vec2 out_uv;

void main() {
    out_uv = vec2((gl_VertexIndex << 1) & 2, gl_VertexIndex & 2);
    gl_Position = vec4(out_uv * 2.0 - 1.0, 0.0, 1.0);
}
