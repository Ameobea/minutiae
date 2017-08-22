mergeInto(LibraryManager.library, {
  /**
   * Given a pointer to the 3D array of floating point data, renders it using WebGL
   */
  buf_render: function(ptr, screenRatio, cameraX, cameraY, cameraZ, focalX, focalY, focalZ) {
    // don't reder if the DOM hasn't been fully initialized yet
    if(!Module.canvas) {
      return;
    }

    // Create the slice looking into Emscripten memory
    var size = Module.canvas.width;
    var buf = new Float32Array(HEAPU8.buffer, ptr, size * size * size);

    // call the raymarcher kernel and update the canvas
    Module.rayMarcherKernel(buf, size, screenRatio, cameraX, cameraY, cameraZ, focalX, focalY, focalZ);
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
