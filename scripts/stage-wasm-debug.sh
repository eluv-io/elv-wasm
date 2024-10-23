#!/bin/bash

if ! command -v wasm-gc &> /dev/null
then
    echo "wasm-gc could not be found, please install via cargo install wasm-gc"
    exit
fi

echo $1 $2

if [ $# -ne 2 ]
then
    echo "Bad number of args"
else
    cp $1/target/wasm32-unknown-unknown/debug/rproxy.wasm $2/bitcode/wasm/rproxy/rproxy.wasm
    cp $1/target/wasm32-unknown-unknown/debug/real_img.wasm $2/bitcode/wasm/image/real_img.wasm
    cp $1/target/wasm32-unknown-unknown/debug/search.wasm $2/bitcode/wasm/search/search.wasm
    cp $1/target/wasm32-unknown-unknown/debug/objtar.wasm $2/bitcode/wasm/objtar/objtar.wasm
    cp $1/target/wasm32-unknown-unknown/debug/parts_download.wasm $2/bitcode/wasm/objtar/parts_download.wasm

    cp $1/samples/library/library.wasm $2/bitcode/wasm/library/library.wasm
    cp $1/samples/proxy/proxy.wasm $2/bitcode/wasm/proxy/proxy.wasm
fi

