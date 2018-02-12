cargo rustc --target=wasm32-unknown-emscripten --release --verbose -- -Z print-link-args -C\
  link-args="-v -g -O3 --js-library ./src/js/library_minutiae.js --closure 1 --llvm-lto 3 -s TOTAL_MEMORY=67108864"
cp target/wasm32-unknown-emscripten/release/ants.wasm .
cp target/wasm32-unknown-emscripten/release/ants.js .
