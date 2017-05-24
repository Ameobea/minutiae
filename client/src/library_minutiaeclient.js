mergeInto(LibraryManager.library, {
  /**
   * Function that sets up and initializes the callbacks for messages on the main WebSocket connection between this
   * client and the server.  Starts the process of receiving messages from the server, processing them, and updating the
   * canvas as well as managing sequence numbers internally etc.
   */
  init_ws: function() {
    console.log('Initilizing websocket connection from the JS side...');
    // socket is global and is initialized to `null`
    socket = new WebSocket('ws://127.0.0.1:7037');

    // Create a client and store a pointer to it in `Module`
    var clientPtr = Module.ccall('create_client', 'number', ['number'], [800]);
    // Get a pointer to its inner pixel data buffer and store that in `Module` as well
    var pixdataPtr = Module.ccall('get_buffer_ptr', 'number', ['number'], [clientPtr]);
    var processMessage = Module.cwrap('process_message', null, ['number', 'number', 'number']);

    function blobToTypedArray(blob, cb) {
      var fileReader = new FileReader();
      fileReader.onload = function() { cb(new Uint8Array(this.result)); };
      fileReader.readAsArrayBuffer(blob);
    }

    /**
     * Given a pointer to an array of pixel data, renders it to the on-screen canvas.
     */
    function canvas_render(ptr) {
      var canvas = Module.canvas;
      var ctx = canvas.getContext('2d');
      var buf = new Uint8ClampedArray(HEAPU8.buffer, ptr, canvas.width * canvas.width * 4);
      var imageData = new ImageData(buf, canvas.width, canvas.width);
      ctx.putImageData(imageData, 0, 0);
    }

    socket.onmessage = function(e) {
      // convert the blog from the websocket message into an `ArrayBuffer`
      blobToTypedArray(e.data, function(ta) {
        // Allocate space in the Emscripten heap for the message's contents, copy it there, and invoke the message handler
        var bufPtr = Module._malloc(ta.length + 1);
        Module.writeArrayToMemory(ta, bufPtr);
        processMessage(clientPtr, bufPtr, ta.length);
        // once the message has been processed, update the canvas from the pixel data buffer.
        canvas_render(pixdataPtr);
        // free the allocated memory to avoid leaking it
        Module._free(bufPtr);
      });
    }

    socket.onerror = function(e) {
      console.error('Error in websocket connection: ');
      console.log(e);
    }

    socket.onopen = function(e) {
      console.log('Successfully opened WebSocket connection to server!');
      // request an initial snapshot from the server with the full universe to start off
      Module.ccall('request_snapshot', null, [null], []);
    }
  },

  /**
   * Given a pointer to a buffer containing a serialized `ClientMessage` to be sent to the server, sends it over
   * the websocket connection and deallocates the buffer.
   */
  send_client_message: function(ptr, len) {
    // create a typed array view into Emscripten's memory at the given index and send it over the websocket
    var buf = new Uint8ClampedArray(HEAP8.buffer, ptr, len);
    socket.send(buf);
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
  }
});
