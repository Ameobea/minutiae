cargo rustc --target=wasm32-unknown-emscripten --release --verbose -- -Z print-link-args -C\
  link-args="-v --profiling -O3 --js-library ./src/js/library_minutae.js --closure 1 --llvm-lto 3 -s TOTAL_MEMORY=67108864 -s SIMD=1"
