cargo rustc --target=wasm32-unknown-emscripten --release --verbose -- -Z print-link-args -C\
  link-args="-v --profiling --js-library ./src/library_minutiae.js -s TOTAL_MEMORY=67108864"
