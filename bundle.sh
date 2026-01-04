#!/bin/sh
cargo nih-plug bundle nam-nih-plug --release
cp -r target/bundled/nam-nih-plug.vst3 ~/.vst3/
cp target/bundled/nam-nih-plug.clap ~/.clap/
