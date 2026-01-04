use nih_plug::prelude::Editor;
use ringbuf::{HeapProd, traits::Producer};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use vizia_plug::vizia::prelude::*;
use vizia_plug::widgets::*;
use vizia_plug::{ViziaState, ViziaTheming, create_vizia_editor};

use crate::{NamParams, neuralaudio};

#[derive(Lens)]
struct GuiData {
    params: Arc<NamParams>,
    sender: Arc<Mutex<HeapProd<(neuralaudio::Model, PathBuf)>>>,
    model_path: PathBuf,
    model_name: String,
}

impl Model for GuiData {
    fn event(&mut self, _cx: &mut EventContext, event: &mut Event) {
        event.map(|gui_event: &GuiEvent, _meta| match gui_event {
            GuiEvent::LoadModel => {
                if let Some(path) = rfd::FileDialog::new()
                    .set_directory(self.model_path.parent().unwrap_or_else(|| Path::new("")))
                    .add_filter("NAM", &["nam"])
                    .pick_file()
                {
                    let model = neuralaudio::Model::from_file(path.clone()).unwrap();
                    self.model_path = path.clone();
                    self.model_name = path
                        .clone()
                        .file_stem()
                        .unwrap()
                        .to_string_lossy()
                        .into_owned();
                    let _ = self.sender.lock().unwrap().try_push((model, path));
                }
            }
        });
    }
}

pub enum GuiEvent {
    LoadModel,
}

pub(crate) fn create(
    params: Arc<NamParams>,
    sender: Arc<Mutex<HeapProd<(neuralaudio::Model, PathBuf)>>>,
) -> Option<Box<dyn Editor>> {
    create_vizia_editor(
        ViziaState::new(|| (400, 500)),
        ViziaTheming::Custom,
        move |cx, _| {
            let model_path = params.model_path.lock().unwrap().clone();
            GuiData {
                params: params.clone(),
                sender: sender.clone(),
                model_name: model_path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned(),
                model_path,
            }
            .build(cx);

            VStack::new(cx, |cx| {
                Label::new(cx, "NAM").font_size(30.0);
                Label::new(cx, "");

                Button::new(cx, |cx| Label::new(cx, "Open a NAM file"))
                    .on_press(|ex| ex.emit(GuiEvent::LoadModel));
                Label::new(cx, GuiData::model_name)
                    .width(Pixels(350.0))
                    .alignment(Alignment::Center);
                Label::new(cx, "");

                Label::new(cx, "Bass");
                ParamSlider::new(cx, GuiData::params, |params| &params.bass);
                Label::new(cx, "Mid");
                ParamSlider::new(cx, GuiData::params, |params| &params.mid);
                Label::new(cx, "Treble");
                ParamSlider::new(cx, GuiData::params, |params| &params.treble);
                Label::new(cx, "Output");
                ParamSlider::new(cx, GuiData::params, |params| &params.output);
            })
            .alignment(Alignment::Center);
        },
    )
}
