use std::os::raw::{c_float, c_int};
use std::path::Path;

#[repr(C)]
pub struct NeuralModel {
    _unused: [u8; 0],
}

#[cfg(windows)]
type WChar = u16;
#[cfg(not(windows))]
type WChar = i32;

unsafe extern "C" {
    fn CreateModelFromFile(model_path: *const WChar) -> *mut NeuralModel;
    fn DeleteModel(model: *mut NeuralModel);
    fn SetLSTMLoadMode(load_mode: c_int);
    fn SetWaveNetLoadMode(load_mode: c_int);
    fn SetAudioInputLevelDBu(audio_dbu: c_float);
    fn SetDefaultMaxAudioBufferSize(max_size: c_int);
    fn GetLoadMode(model: *mut NeuralModel) -> c_int;
    fn IsStatic(model: *mut NeuralModel) -> bool;
    fn SetMaxAudioBufferSize(model: *mut NeuralModel, max_size: c_int);
    fn GetRecommendedInputDBAdjustment(model: *mut NeuralModel) -> c_float;
    fn GetRecommendedOutputDBAdjustment(model: *mut NeuralModel) -> c_float;
    fn GetSampleRate(model: *mut NeuralModel) -> c_float;
    fn Process(model: *mut NeuralModel, input: *mut f32, output: *mut f32, num_samples: usize);
}

pub struct Model {
    inner: *mut NeuralModel,
}

impl Model {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let wide_path = Self::encode_path(path.as_ref());

        let ptr = unsafe { CreateModelFromFile(wide_path.as_ptr()) };

        if ptr.is_null() {
            Err("Failed to load NAM model: CreateModelFromFile returned null".to_string())
        } else {
            Ok(Self { inner: ptr })
        }
    }

    pub fn process(&mut self, input: &[f32], output: &mut [f32]) {
        assert_eq!(
            input.len(),
            output.len(),
            "Input and output buffers must have the same length"
        );
        unsafe {
            Process(
                self.inner,
                input.as_ptr() as *mut f32,
                output.as_mut_ptr(),
                input.len(),
            );
        }
    }

    pub fn get_sample_rate(&self) -> f32 {
        unsafe { GetSampleRate(self.inner) }
    }

    pub fn is_static(&self) -> bool {
        unsafe { IsStatic(self.inner) }
    }

    fn encode_path(path: &Path) -> Vec<WChar> {
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStrExt;
            path.as_os_str().encode_wide().chain(Some(0)).collect()
        }
        #[cfg(not(windows))]
        {
            path.to_string_lossy()
                .chars()
                .map(|c| c as i32)
                .chain(Some(0))
                .collect()
        }
    }
}

impl Drop for Model {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe { DeleteModel(self.inner) };
        }
    }
}

pub fn set_lstm_load_mode(mode: i32) {
    unsafe { SetLSTMLoadMode(mode as c_int) };
}

pub fn set_default_max_audio_buffer_size(size: i32) {
    unsafe { SetDefaultMaxAudioBufferSize(size as c_int) };
}

unsafe impl Send for Model {}
