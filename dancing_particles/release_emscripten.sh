cargo rustc --target=asmjs-unknown-emscripten --release --verbose -- -Z print-link-args -C\
  link-args="-v -O3 --js-library ./src/library_minutiae.js --llvm-lto 3 -s TOTAL_MEMORY=67108864"
