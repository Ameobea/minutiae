mergeInto(LibraryManager.library, {
  /**
   * Given a pointer to the 3D array of floating point data, renders it using WebGL
   */
  buf_render: function(ptr) {
    var canvas = Module.canvas;
    var gl = canvas.getContext('webgl2');
    if (!gl) {
      alert('needs webgl 2.0');
    }

    // Create the slice looking into Emscripten memory
    var size = canvas.width;
    var buf = new Float32Array(HEAPU8.buffer, ptr, size * size * size);

    // initialize the 3D texture
    var texture = gl.createTexture();
    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_3D, texture);
    gl.texParameteri(gl.TEXTURE_3D, gl.TEXTURE_BASE_LEVEL, 0);
    gl.texParameteri(gl.TEXTURE_3D, gl.TEXTURE_MAX_LEVEL, 0);
    gl.texParameteri(gl.TEXTURE_3D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_3D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);

    gl.texImage3D(
      gl.TEXTURE_3D,
      0,
      gl.R32F,
      size,
      size,
      size,
      0,
      gl.RED,
      gl.FLOAT,
      buf
    );

    // copied from https://stackoverflow.com/questions/37586193/support-for-short-3d-texture-in-webgl-2-0
    // FS code:

    var fs = `#version 300 es

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
    `;

    var vs = `#version 300 es
    in vec4 position;
    out vec3 v_texcoord;
    void main() {
      gl_Position = position;
      v_texcoord = vec3(0);
    }
    `

    var programInfo = twgl.createProgramInfo(gl, [vs, fs]);
    var bufferInfo = twgl.primitives.createXYQuadBufferInfo(gl);

    gl.useProgram(programInfo.program);
    twgl.setBuffersAndAttributes(gl, programInfo, bufferInfo);
    twgl.drawBufferInfo(gl, bufferInfo);
  },
});
