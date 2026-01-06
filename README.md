# NAM nih-plug
An audio plugin for **Neural Amp Modeler (NAM)**, built using the [nih-plug](https://github.com/robbert-vdh/nih-plug) plugin framework and [NeuralAudio](https://github.com/mikeoliphant/NeuralAudio) for audio processing with neural model.
## Build
```bash
cargo install --git https://github.com/robbert-vdh/nih-plug.git cargo-nih-plug
git clone https://codeberg.org/lzj15/nam-nih-plug.git
cd nam-nih-plug
cargo nih-plug bundle nam-nih-plug --release
