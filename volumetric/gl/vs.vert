#version 300 es

in vec4 position;
out vec3 v_texcoord;

void main() {
  gl_Position = position;
  v_texcoord = vec3(0);
}
