mergeInto(LibraryManager.library, {
  /**
   * Given a pointer to the 3D array of floating point data, renders it using WebGL
   */
  buf_render: function(ptr) {
    var canvas = Module.canvas;
    var ctx = canvas.getContext('gl2');
    var buf = new Uint8ClampedArray(HEAPU8.buffer, ptr, canvas.width * canvas.width * 4);
    var imageData = new ImageData(buf, canvas.width, canvas.width);
    ctx.putImageData(imageData, 0, 0);
  },
});
