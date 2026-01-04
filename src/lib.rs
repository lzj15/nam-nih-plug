use biquad::*;
use nih_plug::prelude::*;
use ringbuf::traits::{Consumer, Split};
use ringbuf::{HeapCons, HeapProd, HeapRb};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
mod editor;
mod neuralaudio;

const BASS_FREQ: f32 = 500.0;
const MID_FREQ: f32 = 1500.0;
const TREBLE_FREQ: f32 = 3000.0;

pub struct Nam {
    params: Arc<NamParams>,
    model: Option<neuralaudio::Model>,
    output_buffer: Vec<f32>,
    filters: Option<[DirectForm1<f32>; 3]>,
    sender: Option<HeapProd<(neuralaudio::Model, PathBuf)>>,
    receiver: Option<HeapCons<(neuralaudio::Model, PathBuf)>>,
}

#[derive(Params)]
struct NamParams {
    #[id = "bass"]
    pub bass: FloatParam,
    #[id = "mid"]
    pub mid: FloatParam,
    #[id = "treble"]
    pub treble: FloatParam,
    #[id = "output"]
    pub output: FloatParam,
    #[persist = "model-path"]
    pub model_path: Mutex<PathBuf>,
}

impl Default for Nam {
    fn default() -> Self {
        let rb = HeapRb::<(neuralaudio::Model, PathBuf)>::new(2);
        let (prod, cons) = rb.split();
        Self {
            params: Arc::new(NamParams::default()),
            model: None,
            output_buffer: Vec::new(),
            filters: None,
            sender: Some(prod),
            receiver: Some(cons),
        }
    }
}

impl Default for NamParams {
    fn default() -> Self {
        Self {
            bass: FloatParam::new(
                "Bass",
                0.0,
                FloatRange::Linear {
                    min: -20.0,
                    max: 20.0,
                },
            )
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_rounded(2)),

            mid: FloatParam::new(
                "Mid",
                0.0,
                FloatRange::Linear {
                    min: -15.0,
                    max: 15.0,
                },
            )
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_rounded(2)),

            treble: FloatParam::new(
                "Treble",
                0.0,
                FloatRange::Linear {
                    min: -10.0,
                    max: 10.0,
                },
            )
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_rounded(2)),

            output: FloatParam::new(
                "Output",
                util::db_to_gain(0.0),
                FloatRange::Skewed {
                    min: util::db_to_gain(-20.0),
                    max: util::db_to_gain(20.0),
                    factor: FloatRange::gain_skew_factor(-20.0, 20.0),
                },
            )
            .with_smoother(SmoothingStyle::Logarithmic(50.0))
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),

            model_path: Mutex::new(PathBuf::new()),
        }
    }
}

impl Plugin for Nam {
    const NAME: &'static str = "NAM";
    const VENDOR: &'static str = "Zhijian Li";
    const URL: &'static str = "https://codeberg.org/lzj15";
    const EMAIL: &'static str = "lzj15@proton.me";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(2),
        main_output_channels: NonZeroU32::new(2),
        ..AudioIOLayout::const_default()
    }];
    const MIDI_INPUT: MidiConfig = MidiConfig::None;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;
    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        editor::create(
            self.params.clone(),
            Arc::new(Mutex::new(self.sender.take().unwrap())),
        )
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.output_buffer = Vec::with_capacity(buffer_config.max_buffer_size as usize);

        let path = self.params.model_path.lock().unwrap().clone();
        if path.exists() {
            self.model = Some(neuralaudio::Model::from_file(&path).unwrap());
            println!("Loaded {}", path.display());
        }

        self.filters = Some([
            DirectForm1::<f32>::new(
                Coefficients::<f32>::from_params(
                    Type::LowShelf(0.0),
                    buffer_config.sample_rate.hz(),
                    BASS_FREQ.hz(),
                    Q_BUTTERWORTH_F32,
                )
                .unwrap(),
            ),
            DirectForm1::<f32>::new(
                Coefficients::<f32>::from_params(
                    Type::PeakingEQ(0.0),
                    buffer_config.sample_rate.hz(),
                    MID_FREQ.hz(),
                    Q_BUTTERWORTH_F32,
                )
                .unwrap(),
            ),
            DirectForm1::<f32>::new(
                Coefficients::<f32>::from_params(
                    Type::HighShelf(0.0),
                    buffer_config.sample_rate.hz(),
                    TREBLE_FREQ.hz(),
                    Q_BUTTERWORTH_F32,
                )
                .unwrap(),
            ),
        ]);
        true
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        if let Some((model, path)) = self.receiver.as_mut().unwrap().try_pop() {
            self.model = Some(model);
            println!("Loaded {}", path.display());
            *self.params.model_path.lock().unwrap() = path;
        }

        self.output_buffer.resize(buffer.samples(), 0.0);
        if let Some(model) = self.model.as_mut() {
            model.process(buffer.as_slice()[0], self.output_buffer.as_mut_slice());
        } else {
            self.output_buffer.copy_from_slice(buffer.as_slice()[0]);
        }

        let coeffs_bass = Coefficients::from_params(
            Type::LowShelf(self.params.bass.value()),
            context.transport().sample_rate.hz(),
            BASS_FREQ.hz(),
            Q_BUTTERWORTH_F32,
        )
        .unwrap();
        let coeffs_mid = Coefficients::from_params(
            Type::PeakingEQ(self.params.mid.value()),
            context.transport().sample_rate.hz(),
            MID_FREQ.hz(),
            Q_BUTTERWORTH_F32,
        )
        .unwrap();
        let coeffs_treble = Coefficients::from_params(
            Type::HighShelf(self.params.treble.value()),
            context.transport().sample_rate.hz(),
            TREBLE_FREQ.hz(),
            Q_BUTTERWORTH_F32,
        )
        .unwrap();

        for (index, coeffs) in [coeffs_bass, coeffs_mid, coeffs_treble].iter().enumerate() {
            self.filters.as_mut().unwrap()[index].update_coefficients(*coeffs);
        }

        for sample in self.output_buffer.as_mut_slice() {
            for filter in self.filters.as_mut().unwrap() {
                *sample = filter.run(*sample);
            }
            *sample *= self.params.output.smoothed.next();
        }

        buffer.as_slice()[0].copy_from_slice(&self.output_buffer);
        buffer.as_slice()[1].copy_from_slice(&self.output_buffer);

        ProcessStatus::Normal
    }

    fn deactivate(&mut self) {}
}

impl ClapPlugin for Nam {
    const CLAP_ID: &'static str = "org.codeberg.lzj15.nam-nih-plug";
    const CLAP_DESCRIPTION: Option<&'static str> = None;
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::AudioEffect];
}

impl Vst3Plugin for Nam {
    const VST3_CLASS_ID: [u8; 16] = *b"nam-nih-plug0000";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[Vst3SubCategory::Fx];
}

nih_export_clap!(Nam);
nih_export_vst3!(Nam);
