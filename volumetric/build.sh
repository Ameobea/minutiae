cd engine && ./asmjs.sh
cd ..

mkdir -p dist
cp ./engine/target/asmjs-unknown-emscripten/debug/volumetric.js ./dist/volumetric.js
