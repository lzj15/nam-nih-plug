#!/bin/sh
cargo build --release
cp target/x86_64-unknown-linux-musl/release/libnam_nih_plug.so target/release/
cargo nih-plug bundle nam-nih-plug --release
cp -r target/bundled/nam-nih-plug.vst3 ~/.vst3/
cp target/bundled/nam-nih-plug.clap ~/.clap/
