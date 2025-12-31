use biquad::*;
use nih_plug::prelude::*;
use std::sync::Arc;
mod neuralaudio;

const BASS_FREQ: f32 = 500.0;
const MID_FREQ: f32 = 1500.0;
const TREBLE_FREQ: f32 = 3000.0;

struct Nam {
    params: Arc<NamParams>,
    amp: Option<neuralaudio::Model>,
    temp_buffer: Vec<f32>,
    filters: Option<[DirectForm1<f32>; 3]>,
}

/// The [`Params`] derive macro gathers all of the information needed for the wrapper to know about
/// the plugin's parameters, persistent serializable fields, and nested parameter groups. You can
/// also easily implement [`Params`] by hand if you want to, for instance, have multiple instances
/// of a parameters struct for multiple identical oscillators/filters/envelopes.
#[derive(Params)]
struct NamParams {
    /// The parameter's ID is used to identify the parameter in the wrapped plugin API. As long as
    /// these IDs remain constant, you can rename and reorder these fields as you wish. The
    /// parameters are exposed to the host in the same order they were defined. In this case, this
    /// gain parameter is stored as linear gain while the values are displayed in decibels.
    #[id = "bass"]
    pub bass: FloatParam,
    #[id = "mid"]
    pub mid: FloatParam,
    #[id = "treble"]
    pub treble: FloatParam,
    #[id = "output"]
    pub output: FloatParam,
}

impl Default for Nam {
    fn default() -> Self {
        Self {
            params: Arc::new(NamParams::default()),
            amp: None,
            temp_buffer: Vec::new(),
            filters: None,
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
            .with_unit(" dB"),

            mid: FloatParam::new(
                "Mid",
                0.0,
                FloatRange::Linear {
                    min: -15.0,
                    max: 15.0,
                },
            )
            .with_unit(" dB"),

            treble: FloatParam::new(
                "Treble",
                0.0,
                FloatRange::Linear {
                    min: -10.0,
                    max: 10.0,
                },
            )
            .with_unit(" dB"),

            // This gain is stored as linear gain. NIH-plug comes with useful conversion functions
            // to treat these kinds of parameters as if we were dealing with decibels. Storing this
            // as decibels is easier to work with, but requires a conversion for every sample.
            output: FloatParam::new(
                "Output",
                util::db_to_gain(0.0),
                FloatRange::Skewed {
                    min: util::db_to_gain(-20.0),
                    max: util::db_to_gain(20.0),
                    // This makes the range appear as if it was linear when displaying the values as
                    // decibels
                    factor: FloatRange::gain_skew_factor(-20.0, 20.0),
                },
            )
            // Because the gain parameter is stored as linear gain instead of storing the value as
            // decibels, we need logarithmic smoothing
            .with_smoother(SmoothingStyle::Logarithmic(50.0))
            .with_unit(" dB")
            // There are many predefined formatters we can use here. If the gain was stored as
            // decibels instead of as a linear gain value, we could have also used the
            // `.with_step_size(0.1)` function to get internal rounding.
            .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),
        }
    }
}

impl Plugin for Nam {
    const NAME: &'static str = "NAM";
    const VENDOR: &'static str = "Zhijian Li";
    // You can use `env!("CARGO_PKG_HOMEPAGE")` to reference the homepage field from the
    // `Cargo.toml` file here
    const URL: &'static str = "https://codeberg.org/lzj15";
    const EMAIL: &'static str = "lzj15@proton.me";

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    // The first audio IO layout is used as the default. The other layouts may be selected either
    // explicitly or automatically by the host or the user depending on the plugin API/backend.
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(2),
        main_output_channels: NonZeroU32::new(2),
        ..AudioIOLayout::const_default()
    }];

    const MIDI_INPUT: MidiConfig = MidiConfig::None;
    // Setting this to `true` will tell the wrapper to split the buffer up into smaller blocks
    // whenever there are inter-buffer parameter changes. This way no changes to the plugin are
    // required to support sample accurate automation and the wrapper handles all of the boring
    // stuff like making sure transport and other timing information stays consistent between the
    // splits.
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    // If the plugin can send or receive SysEx messages, it can define a type to wrap around those
    // messages here. The type implements the `SysExMessage` trait, which allows conversion to and
    // from plain byte buffers.
    type SysExMessage = ();
    // More advanced plugins can use this to run expensive background tasks. See the field's
    // documentation for more information. `()` means that the plugin does not have any background
    // tasks.
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    // This plugin doesn't need any special initialization, but if you need to do anything expensive
    // then this would be the place. State is kept around when the host reconfigures the
    // plugin. If we do need special initialization, we could implement the `initialize()` and/or
    // `reset()` methods

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.temp_buffer = Vec::with_capacity(buffer_config.max_buffer_size as usize);

        self.amp = Some(
            neuralaudio::Model::from_file(
                "/home/lzj/Data/music/NAM/[AMP] PRS-MT100 LEAD Noon SM57 - STD.nam",
            )
            .expect("Failed to load model"),
        );

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
        self.temp_buffer = vec![0.0; buffer.samples()];
        self.temp_buffer
            .copy_from_slice(buffer.as_slice_immutable()[0]);
        self.amp
            .as_mut()
            .unwrap()
            .process(self.temp_buffer.as_slice(), buffer.as_slice()[0]);

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

        for sample in buffer.as_slice()[0].as_mut() {
            for filter in self.filters.as_mut().unwrap() {
                *sample = filter.run(*sample);
            }
            // Smoothing is optionally built into the parameters themselves
            *sample *= self.params.output.smoothed.next();
        }

        self.temp_buffer
            .copy_from_slice(buffer.as_slice_immutable()[0]);
        buffer.as_slice()[1].copy_from_slice(&self.temp_buffer);

        ProcessStatus::Normal
    }

    // This can be used for cleaning up special resources like socket connections whenever the
    // plugin is deactivated. Most plugins won't need to do anything here.
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
