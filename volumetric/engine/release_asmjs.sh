cargo rustc --target=asmjs-unknown-emscripten --release --verbose -- -Z print-link-args -C\
  link-args="-v -g -O3 --js-library ./src/library_minutiae.js --closure 1 --llvm-lto 3 -s TOTAL_MEMORY=1073741824"
