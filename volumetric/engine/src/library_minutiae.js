mergeInto(LibraryManager.library, {
  /**
   * Given a pointer to the 3D array of floating point data, renders it using WebGL
   */
  buf_render: function(ptr) {
    if(!Module.vs || !Module.fs) {
      // wait for shader source code to be retrieved before trying to render anything
      return;
    }

    var canvas = Module.canvas;
    var gl = canvas.getContext('webgl2');
    if (!gl) {
      alert('needs webgl 2.0');
      return;
    }

    if(!Module.shadersCompiled) {
      var programInfo = twgl.createProgramInfo(gl, [Module.vs, Module.fs]);
      var bufferInfo = twgl.primitives.createXYQuadBufferInfo(gl);

      gl.useProgram(programInfo.program);
      twgl.setBuffersAndAttributes(gl, programInfo, bufferInfo);

      Module.shadersCompiled = true;
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

    twgl.drawBufferInfo(gl, bufferInfo);
  },
});
