use crate::localization::helper::fl;
use crate::ui::video_offsets_dialog::messages::{
    internal::PrivateMsg, VideoOffsetsComponentInput, VideoOffsetsComponentOutput,
};
use crate::ui::video_offsets_dialog::model::{VideoOffsetsComponentInit, VideoOffsetsComponentModel};
use mxl_relm4_components::relm4::{self, adw::prelude::*, prelude::*};

const CONVERSION_RATE: f64 = 1000000_f64;
const OFFSET_MIN: f64 = -1500_f64;
const OFFSET_MAX: f64 = 1500_f64;
const OFFSET_INCREMENT_STEP: f64 = 1_f64;
const OFFSET_PAGE_INCREMENT: f64 = 100_f64;
const OFFSET_PAGE_SIZE: f64 = 0_f64;

#[relm4::component(pub)]
impl Component for VideoOffsetsComponentModel {
    type Init = VideoOffsetsComponentInit;
    type Input = VideoOffsetsComponentInput;
    type Output = VideoOffsetsComponentOutput;
    type CommandOutput = ();

    view! {
        adw::PreferencesWindow {
            set_title: Some(&fl!("video-offsets")),
            set_hide_on_close: true,
            set_destroy_with_parent: true,

            add = &adw::PreferencesPage {
                set_vexpand: true,

                add = &adw::PreferencesGroup {
                    adw::ActionRow {
                        set_hexpand: true,
                        set_title: &fl!("video-offsets-audio"),
                        set_subtitle: &fl!("video-offsets-audio", "description"),

                        add_suffix = &gtk::SpinButton {
                            set_halign: gtk::Align::End,
                            set_valign: gtk::Align::Center,
                            #[watch]
                            #[block_signal(audio_video_offset_changed_handler)]
                            set_adjustment: &gtk::Adjustment::new(model.audio_video_offset as f64 / CONVERSION_RATE,
                                                                  OFFSET_MIN,
                                                                  OFFSET_MAX,
                                                                  OFFSET_INCREMENT_STEP,
                                                                  OFFSET_PAGE_INCREMENT,
                                                                  OFFSET_PAGE_SIZE),
                            connect_value_changed[sender] => move |spin_button| {
                                sender
                                    .input_sender()
                                    .send(VideoOffsetsComponentInput::PrivateMessage(
                                        PrivateMsg::AudioVideoOffsetChanged((spin_button.value() * CONVERSION_RATE) as i64),
                                    ))
                                    .unwrap_or_default();
                            } @audio_video_offset_changed_handler,
                        },
                    },

                    adw::ActionRow {
                        set_hexpand: true,
                        set_title: &fl!("video-offsets-subtitle"),
                        set_subtitle: &fl!("video-offsets-subtitle", "description"),

                        add_suffix = &gtk::SpinButton {
                            set_halign: gtk::Align::End,
                            set_valign: gtk::Align::Center,
                            #[watch]
                            #[block_signal(subtitle_video_offset_changed_handler)]
                            set_adjustment: &gtk::Adjustment::new(model.subtitle_video_offset as f64 / CONVERSION_RATE,
                                                                  OFFSET_MIN,
                                                                  OFFSET_MAX,
                                                                  OFFSET_INCREMENT_STEP,
                                                                  OFFSET_PAGE_INCREMENT,
                                                                  OFFSET_PAGE_SIZE),
                            connect_value_changed[sender] => move |spin_button| {
                                sender
                                    .input_sender()
                                    .send(VideoOffsetsComponentInput::PrivateMessage(
                                        PrivateMsg::SubtitleVideoOffsetChanged((spin_button.value() * CONVERSION_RATE) as i64),
                                    ))
                                    .unwrap_or_default();
                            } @subtitle_video_offset_changed_handler,
                        },
                    },
                },
            }
        }
    }

    // Initialize the component.
    fn init(init: Self::Init, root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let model = VideoOffsetsComponentModel {
            audio_video_offset: init.audio_video_offset,
            subtitle_video_offset: init.subtitle_video_offset,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        msg: Self::Input,
        sender: ComponentSender<Self>,
        _: &Self::Root,
    ) {
        match msg {
            VideoOffsetsComponentInput::SetAudioVideoOffset(offset) => self.audio_video_offset = offset,
            VideoOffsetsComponentInput::SetSubtitleVideoOffset(offset) => self.subtitle_video_offset = offset,
            VideoOffsetsComponentInput::PrivateMessage(msg) => match msg {
                PrivateMsg::AudioVideoOffsetChanged(offset) => {
                    sender
                        .output(VideoOffsetsComponentOutput::SetAudioVideoOffset(offset))
                        .unwrap_or_default();
                }
                PrivateMsg::SubtitleVideoOffsetChanged(offset) => {
                    sender
                        .output(VideoOffsetsComponentOutput::SetSubtitleVideoOffset(offset))
                        .unwrap_or_default();
                }
            },
        }
        self.update_view(widgets, sender)
    }
}

impl VideoOffsetsComponentModel {
    pub fn audio_video_offset(&self) -> i64 {
        self.audio_video_offset
    }

    pub fn subtitle_video_offset(&self) -> i64 {
        self.subtitle_video_offset
    }
}
