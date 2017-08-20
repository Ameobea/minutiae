#version 300 es

// mostly copied from:
// https://stackoverflow.com/questions/37586193/support-for-short-3d-texture-in-webgl-2-0

precision highp float;
precision highp int;
precision highp isampler3D;

uniform isampler3D textureData;

in vec3 v_texcoord;

out vec4 color;

void main()
{
   /*ivec4 value = texture(textureData, v_texcoord);
   if( value.x == 0 )
      color = vec4(1.0, 0.0, 0.0, 1.0);
   else if( value.x == 1)
      color = vec4(1.0, 1.0, 0.0, 1.0);
   else if( value.x < 0 )
      color = vec4(0.0, 0.0, 1.0, 1.0);
   else*/
      color = vec4(1.0,0.0,0.0,1.0);
}
