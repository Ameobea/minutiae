cargo rustc --target=wasm32-unknown-emscripten --verbose -- -Z print-link-args -C\
  link-args="-v -g --js-library ./src/library_minutiae.js -s TOTAL_MEMORY=67108864"
