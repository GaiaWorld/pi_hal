#version 450
    
precision highp float;

// glyph_vertex_t: x, y; g16hi, g16lo; 
layout (location = 0) in vec4 a_glyph_vertex; // 顶点、uv
layout (location = 1) in vec4 info; // sdf信息;


layout (location = 0) out vec2 uv;
layout (location = 1) out vec4 u_info;

void main() {
    vec2 pos = vec2(a_glyph_vertex.x, a_glyph_vertex.y);
    vec4 pos1 = vec4(pos.x , pos.y, 0.0, 1.0);

    gl_Position = pos1;
    uv = a_glyph_vertex.zw;
    u_info = info;
}
