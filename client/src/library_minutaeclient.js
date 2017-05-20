mergeInto(LibraryManager.library, {
  /**
   * Function that sets up and initializes the callbacks for messages on the main WebSocket connection between this
   * client and the server.  Starts the process of receiving messages from the server, processing them, and updating the
   * canvas as well as managing sequence numbers internally etc.
   */
  init_ws: function() {
    // socket is global and is initialized to `null`
    socket = new WebSocket('ws://localhost:7037');

    // Create a client and store a pointer to it in `Module`
    Module.client = null; // TODO
    // Get a pointer to its inner pixel data buffer and store that in `Module` as well
    // TODO

    socket.onmessage = function(e) {
      // Allocate space in the Emscripten heap for the message's contents, copy it there, and invoke the message handler
      let buf = Module._malloc(e.data.length + 1);
      Module.writeArrayToMemory(e.data, buf);

    }

    socket.onerror = function(e) {
      // TODO
    }
  },
  /**
   * Given a pointer to a buffer containing a serialized `ClientMessage` to be sent to the server, sends it over
   * the websocket connection and deallocates the buffer.
   */
  send_client_message: function(pointer, length) {
    // TODO
  },
});
