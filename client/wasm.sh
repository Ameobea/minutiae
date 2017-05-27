cargo rustc --target=wasm32-unknown-emscripten --release --verbose -- -Z print-link-args -C\
  link-args="-v -g -O3 --js-library ./src/library_minutiaeclient.js --closure 1 --llvm-lto 3 -s TOTAL_MEMORY=67108864
  -s EXPORTED_FUNCTIONS=[\"_main\",\"_rust_eh_personality\",\"_create_client\",\"_get_buffer_ptr\",\"_process_message\",\"_request_snapshot\"]"

WASM=`find target/wasm32-unknown-emscripten/release/deps | grep client-.*\.wasm`
# If binaryen is available locally, use it to further optimize the .wast file emitted from emscripten
if hash wasm-opt 2>/dev/null; then
	WAST=`find target/wasm32-unknown-emscripten/release/deps | grep client-.*\.wast`
	wasm-opt $WAST -O3 --print > opt.wast
	wasm-as opt.wast > $WASM
	rm opt.wast
fi

cp $WASM .
