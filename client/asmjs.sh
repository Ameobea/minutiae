cargo rustc --target=asmjs-unknown-emscripten --release --verbose -- -Z print-link-args -C\
  link-args="-v -g -O3 --js-library ./src/library_minutaeclient.js --closure 1 --llvm-lto 3 -s TOTAL_MEMORY=67108864
  -s EXPORTED_FUNCTIONS=[\"_main\",\"_rust_eh_personality\",\"_create_client\",\"_get_buffer_ptr\",\"_process_message\"]"
