mergeInto(LibraryManager.library, {
  canvas_render: function(ptr) {
    var canvas = Module.canvas;
    var ctx = canvas.getContext('2d');
    var buf = new Uint8ClampedArray(HEAPU8.buffer, ptr, canvas.width * canvas.width * 4);
    var imageData = new ImageData(buf, canvas.width, canvas.width);
    ctx.putImageData(imageData, 0, 0);
  },
   /**
   * Wrappers around `console.log` and `console.error` that circumvents the emulated stdout so it can be used after main exits
   */
  js_debug: function(strPtr) {
    var string = Module.Pointer_stringify(strPtr);
    console.log(string);
  },
  js_error: function(strPtr) {
    var string = Module.Pointer_stringify(strPtr);
    console.error(string);
  },
});
