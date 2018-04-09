cargo rustc --target=asmjs-unknown-emscripten --release -- -C \
  link-args="-v -g -O3 --js-library ./src/library_minutiaeclient.js --closure 1 --llvm-lto 3 -s TOTAL_MEMORY=67108864 -s NO_EXIT_RUNTIME=1
  -s EXPORTED_FUNCTIONS=[\"_main\",\"_rust_eh_personality\",\"_create_client\",\"_get_buffer_ptr\",\"_process_message\",\"_request_snapshot\"]
  -s ERROR_ON_UNDEFINED_SYMBOLS=1 -s EXTRA_EXPORTED_RUNTIME_METHODS=[\"Pointer_stringify\",\"ccall\",\"cwrap\",\"_malloc\",\"writeArrayToMemory\",\"_free\"]"
