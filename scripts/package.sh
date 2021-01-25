#!/bin/bash

# make sure it's compiled
cargo build
#cargo build --release
cargo build --target x86_64-unknown-linux-musl
#cargo build --release --target x86_64-unknown-linux-musl

# copy file into package
cp target/debug/clf ./package
cp target/x86_64-unknown-linux-musl/debug/clf ./package/clf.musl
cp target/release/integration_test ./package
cp target/x86_64-unknown-linux-musl/release/integration_test ./package/integration_test.musl

# zip all
rm package/clf.zip
zip -r ./package/clf.zip ./package ./tests/integration -x './tests/integration/tmp*' -x './tests/integration/linux*' -x './tests/integration/ruby*' -x './tests/integration/logfiles*' -x './tests/integration/*.rs' -x './tests/integration/clf.log'

