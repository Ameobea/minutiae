mergeInto(LibraryManager.library, {
  canvas_render: function(ptr) {
    var canvas = Module.canvas;
    var ctx = canvas.getContext('2d');
    // var imageData = ctx.getImageData(0, 0, canvas.width, canvas.height);
    // var data = imageData.data;

    var buf = new Uint8ClampedArray(HEAPU8.buffer, ptr, canvas.width * canvas.width * 4);
    var imageData = new ImageData(buf, canvas.width, canvas.width);

    // for(var i=0; i<data.length; i+=4) {
    //   var offset = ptr + i;
    //   data[i] = HEAPU8[offset];
    //   data[i + 1] = HEAPU8[offset + 1];
    //   data[i + 2] = HEAPU8[offset + 2];
    // }

    ctx.putImageData(imageData, 0, 0);
  },
});
