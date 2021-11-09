#version 310 es

precision highp float;
precision highp sampler;

layout(set=0, binding=1) uniform texture2D t_texture;
layout(set=0, binding=2) uniform sampler s_sampler;

layout(location=0) in vec2 v_tex_coord;
layout(location=0) out vec4 f_color;
layout(location=1) out vec4 f_capture_stream;

void main() {
  f_color = texture(sampler2D(t_texture, s_sampler), v_tex_coord);
  f_capture_stream = f_color;
}
