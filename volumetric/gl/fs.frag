#version 300 es

// mostly copied from:
// https://stackoverflow.com/questions/37586193/support-for-short-3d-texture-in-webgl-2-0

precision highp float;
precision highp int;
precision mediump sampler3D;

uniform sampler3D textureData;

in vec2 fragCoord;

out vec4 color;

void main() {
   // float value = texture(textureData, coord);
   float value = texture(textureData, vec3(fragCoord, 0.)).r;
   color = vec4(value, value, value, 1.);
}
