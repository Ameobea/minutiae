mergeInto(LibraryManager.library, {
  rayMarcherKernel = gpu.createKernel(function(buf, cameraX, cameraY, cameraZ, focalX, focalY, focalZ) {
    var maxSteps = 128;
    var stepSize = 1.0;

    var x = this.thread.x;
    var y = this.thread.y;

    var curX = cameraX;
    var curY = cameraY;
    var curZ = cameraZ;

    // TODO: Calculate coordinates of our pixel within the virtual screen.
    //       It's somewhere in front of the camera; calculate vector between the camera
    //       and the origin and the focal point.

    // TODO: Calculate the vector between the camera and the virtual screen coord.
    //       This vector will be added to the current coords iteratively to perform the raymarch.

    var acc = 0;
    for(var i=0; i<maxSteps; i++) {
      // get the value for the current coordinate from the data buffer

      // accumulate the color/intensity value

      // increment the current coordinates 
    }

    this.color(0.0, 1.0, 0.5, 1.0); // TODO
  }).setOutput([Module.canvas.width, Module.canvas.width]).setGraphical(true),
  /**
   * Given a pointer to the 3D array of floating point data, renders it using WebGL
   */
  buf_render: function(ptr) {
    // Create the slice looking into Emscripten memory
    var size = Module.canvas.width;
    var buf = new Float32Array(HEAPU8.buffer, ptr, size * size * size);

    // TODO: Call the raymarcher kernel and render the created canvas
  },
});
