#!/bin/bash

echo "$1" "$2" "$3"

build_type="release"
if [ "$3" = "true" ]; then
    build_type="debug"
fi
if [ $# -ne 3 ]; then
    echo "Bad number of args"
    echo "Usage:   $0 SOURCEDIR TARGETDIR DEBUG(true|false)"
    echo "Example: $0 . ../content-fabric false"
else
    cp "$1"/target/wasm32-unknown-unknown/"$build_type"/rproxy.wasm "$2"/bitcode/wasm/rproxy/rproxy.wasm
    cp "$1"/target/wasm32-unknown-unknown/"$build_type"/real_img.wasm "$2"/bitcode/wasm/image/real_img.wasm
    cp "$1"/target/wasm32-unknown-unknown/"$build_type"/search.wasm "$2"/bitcode/wasm/search/search.wasm
    cp "$1"/target/wasm32-unknown-unknown/"$build_type"/objtar.wasm "$2"/bitcode/wasm/objtar/objtar.wasm
    cp "$1"/target/wasm32-unknown-unknown/"$build_type"/lro.wasm "$2"/bitcode/wasm/lro/lro.wasm
    cp "$1"/target/wasm32-unknown-unknown/"$build_type"/external.wasm "$2"/bitcode/wasm/external/external.wasm
    cp "$1"/target/wasm32-unknown-unknown/"$build_type"/external_img.wasm "$2"/bitcode/wasm/image/external_img.wasm
    cp "$1"/target/wasm32-unknown-unknown/"$build_type"/panic.wasm "$2"/bitcode/wasm/panic/panic.wasm
    cp "$1"/target/wasm32-unknown-unknown/"$build_type"/parts_download.wasm "$2"/bitcode/wasm/objtar/parts_download.wasm


    if [ "${BUILD_ASC}" = "true" ]; then
        cp -u -f "$1"/target/wasm32-unknown-unknown/release/test_wapc.wasm "$2"/exeng/tests

        cp "$1"/samples/proxy/proxy.wasm "$2"/bitcode/wasm/proxy/proxy.wasm
    fi
fi
