#version 310 es

precision highp float;

layout(location=0) in vec2 a_position;
layout(location=1) in vec2 a_tex_coord;

layout(location=0) out vec2 v_tex_coord;

void main() {
  v_tex_coord = a_tex_coord;
  gl_Position = vec4(a_position, 0.0, 1.0);
}
