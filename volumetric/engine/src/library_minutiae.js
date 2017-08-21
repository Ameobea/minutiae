mergeInto(LibraryManager.library, {
  rayMarcherKernel: function(){
    Module.gpu.createKernel(function(buf, universeSize, screenRatio, cameraX, cameraY, cameraZ, focalX, focalY, focalZ) {
      var maxSteps = 1028;
      // number of universe coordinates traversed per step of the raymarch
      var stepSize = 1.0;

      var x = this.thread.x;
      var y = this.thread.y;
      var unitsPerPx = 2 / universeSize;

      // calculate coordinates of our pixel within the virtual screen.
      // the buffer will be rendered from (-1, -1, -1) to (1, 1, 1) with the center at (0, 0, 0)
      var targetX = focalX + (1 - (x * screenRatio * unitsPerPx));
      var targetY = focalY + (1 - (y * screenRatio * unitsPerPx));
      var targetZ = focalZ + (1 - (y * screenRatio * unitsPerPx));

      // calculate the vector that defines the direction the camera is facing
      var viewVecX = targetX - cameraX;
      var viewVecY = targetY - cameraY;
      var viewVecZ = targetZ - cameraZ;

      // find the largest element of the view vector for use in calculating the step vector
      var largestElem = viewVecX;
      if(Math.abs(largestElem) < Math.abs(viewVecY)) {
        largestElem = viewVecY;
      } else if(Math.abs(largestElem) < Math.abs(viewVecZ)) {
        largestElem = viewVecZ;
      }

      // normalize the view vector by the largest element
      // the largest element of the step vector will not be `stepSize`
      var mult = stepSize / largestElem;
      var stepX = viewVecX * mult;
      var stepY = viewVecY * mult;
      var stepZ = viewVecZ * mult;

      var curX = targetX;
      var curY = targetY;
      var curZ = targetZ;

      var opacity = 0.0;
      for(var i=0; i<maxSteps; i++) {
        // make sure that we're within the bounds of the universe
        if(curX >= -1.0 && curX < 1.0 && curY >= -1.0 && curY < 1.0 && curZ >= -1.0 && curZ < 1.0) {
          // get the index within the 3D buffer
          var normalizedX = Math.floor((curX / unitsPerPx) + 1.0);
          var normalizedY = Math.floor((curY / unitsPerPx) + 1.0);
          var normalizedZ = Math.floor((curZ / unitsPerPx) + 1.0);
          var bufIndex = (normalizedY * universeSize * universeSize) + (normalizedX * universeSize) + normalizedZ;
          // get the value of the coordinate from within the buffer
          var value = buf[bufIndex];
          // accumulate the color/intensity value
          opacity += value;
          // check to see if we've reached opaqueness
          if(opacity >= 1.0) {
            opacity = 1.0;
            break;
          }
        }

        // increment the current coordinates by the step
        curX += stepX;
        curY += stepY;
        curZ += stepZ;
      }

      this.color(opacity, opacity, opacity, 1.0); // TODO
    }).setOutput([Module.canvas.width, Module.canvas.width]).setGraphical(true)
  }(),
  /**
   * Given a pointer to the 3D array of floating point data, renders it using WebGL
   */
  buf_render: function(ptr, screenRatio, cameraX, cameraY, cameraZ, focalX, focalY, focalZ) {
    // Create the slice looking into Emscripten memory
    var size = Module.canvas.width;
    var buf = new Float32Array(HEAPU8.buffer, ptr, size * size * size);

    // call the raymarcher kernel and render the created canvas
    Module.rayMarcherKernel(buf, size, screenRatio, cameraX, cameraY, cameraZ, focalX, focalY, focalZ);
    // set the newly created canvas into the DOM
    Module.canvas.parentNode.replaceChild(Module.rayMarcherKernel.getCanvas(), Module.canvas);
  },
});
