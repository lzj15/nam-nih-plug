fn main() {
    cc::Build::new()
        .cpp(true)
        .file("NeuralAudio/NeuralAudioCAPI/NeuralAudioCApi.cpp")
        .file("NeuralAudio/NeuralAudio/NeuralModel.cpp")
        .include("NeuralAudio/NeuralAudio")
        .include("NeuralAudio/deps/RTNeural/")
        .include("NeuralAudio/deps/RTNeural/modules/json/")
        .include("NeuralAudio/deps/RTNeural/modules/Eigen/")
        .include("NeuralAudio/deps/math_approx/include/")
        .define("RTNEURAL_USE_EIGEN", "1")
        .compile("neuralaudio");
}
