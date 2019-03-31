cargo rustc --target=wasm32-unknown-emscripten --verbose -- -Z print-link-args -C \
  link-args="-v -g --js-library ./src/library_minutiae.js -s TOTAL_MEMORY=1073741824 -s EXTRA_EXPORTED_RUNTIME_METHODS=[\"Pointer_stringify\"]"
