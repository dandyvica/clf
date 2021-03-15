#!/bin/bash
target=release
mkdir ./package

# make sure it's compiled
#cargo build
cargo build --release
#cargo build --target x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl

# copy file into package
cp target/$target/clf ./package
strip target/x86_64-unknown-linux-musl/$target/clf
cp target/x86_64-unknown-linux-musl/$target/clf ./package/clf.musl
#cp target/$target/integration_test ./package
strip target/x86_64-unknown-linux-musl/$target/integration_test
cp target/x86_64-unknown-linux-musl/$target/integration_test ./package/integration_test.musl

# zip all
rm ./package/clf.zip
zip -r ./package/clf.zip ./package/clf.musl ./package/integration_test.musl ./tests/integration -x './tests/integration/tmp*' -x './tests/integration/linux*' -x './tests/integration/ruby*' -x './tests/integration/logfiles*' -x './tests/integration/*.rs' -x './tests/integration/clf.log'

