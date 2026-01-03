#!/bin/sh
cargo build --release
zig cc -shared -o target/nam-nih-plug.so -fPIC -lunwind /usr/lib/libc.a -Wl,-Bstatic -lc++ -lwayland-client -lffi -Wl,--whole-archive target/release/libnam_nih_plug.a

mkdir -p ~/.vst3/nam-nih-plug.vst3/Contents/x86_64-linux/
cp target/nam-nih-plug.so ~/.vst3/nam-nih-plug.vst3/Contents/x86_64-linux/nam-nih-plug.so
cp target/nam-nih-plug.so ~/.clap/nam-nih-plug.clap
