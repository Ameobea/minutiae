cargo rustc --target=wasm32-unknown-emscripten --verbose -- -Z print-link-args -C\
  link-args="-v -g -O0 --js-library ./src/js/library_minutiae.js --closure 0 --llvm-lto 0 -s TOTAL_MEMORY=67108864"
cp target/wasm32-unknown-emscripten/debug/ants.wasm .
