use super::{
    messages::{internal::PrivateMsg, MessageDialogInput, MessageDialogOutput, MessageDialogType},
    model::MessageDialog,
};
use crate::localization::helper::fl;
use mxl_relm4_components::relm4::{self, adw::gtk::prelude::*, prelude::*};

#[relm4::component(pub)]
impl Component for MessageDialog {
    type Init = ();
    type Input = MessageDialogInput;
    type Output = MessageDialogOutput;
    type CommandOutput = ();

    view! {
        #[name(dialog)]
        adw::Window {
            #[watch]
            set_title: Some(model.title.as_str()),
            set_modal: true,
            set_hide_on_close: true,
            set_destroy_with_parent: true,
            set_height_request: 500,
            set_width_request: 800,
            #[watch]
            set_visible: !model.hidden,

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,

                adw::HeaderBar {
                    set_show_end_title_buttons: false,
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_margin_all: 8,
                    set_spacing: 8,

                    gtk::ScrolledWindow {
                        set_vexpand: true,
                        set_hexpand: true,
                        set_vscrollbar_policy: gtk::PolicyType::Always,

                        gtk::TextView {
                            set_buffer: Some(&model.text_buffer),
                            set_vexpand: true,
                            set_hexpand: true,
                            set_editable: false,
                            set_wrap_mode: gtk::WrapMode::Word,
                            set_cursor_visible: false,
                        },
                    },

                    gtk::Box {
                        set_hexpand: true,
                        set_homogeneous: true,
                        set_spacing: 8,

                        gtk::Button {
                            #[watch]
                            set_label: &fl!("create-report"),
                            set_hexpand: true,
                            connect_clicked => MessageDialogInput::PrivateMessage(PrivateMsg::CreateReportButtonPressed),
                        },

                        gtk::Button {
                            #[watch]
                            set_label: match model.dialog_type {
                                MessageDialogType::Fatal => fl!("quit").to_owned(),
                                _ =>  fl!("close").to_owned(),
                            }.as_ref(),
                            add_css_class: "error",
                            set_hexpand: true,
                            connect_clicked => MessageDialogInput::PrivateMessage(PrivateMsg::CloseButtonPressed),
                       },
                    },
                },
            },
        }
    }

    fn init(_init: (), root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let model = MessageDialog {
            hidden: true,
            dialog_type: MessageDialogType::Error,
            title: "".to_owned(),
            text_buffer: gtk::TextBuffer::new(None),
        };
        let widgets = view_output!();

        {
            let controller = gtk::EventControllerKey::new();
            controller.connect_key_pressed(move |_, key, _keycode, _modifiers| {
                if key == gtk::gdk::Key::Escape {
                    sender.input(MessageDialogInput::PrivateMessage(PrivateMsg::CloseButtonPressed));
                    return gtk::glib::Propagation::Stop;
                }
                gtk::glib::Propagation::Proceed
            });
            root.add_controller(controller);
        }

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match msg {
            MessageDialogInput::PrivateMessage(msg) => match msg {
                PrivateMsg::CreateReportButtonPressed => {
                    sender.output_sender().emit(MessageDialogOutput::CreateReport);
                }
                PrivateMsg::CloseButtonPressed => {
                    self.hidden = true;
                    root.close();
                    if self.dialog_type == MessageDialogType::Fatal {
                        sender.output_sender().emit(MessageDialogOutput::Quit);
                    }
                }
            },
            MessageDialogInput::Message(message_type, title, text) => {
                self.title = {
                    if let Some(title) = title {
                        title
                    } else {
                        match message_type {
                            MessageDialogType::Fatal => fl!("fatal-error-title").to_owned(),
                            MessageDialogType::Error => fl!("error-title").to_owned(),
                            MessageDialogType::Warning => fl!("warning-title").to_owned(),
                        }
                    }
                };
                self.dialog_type = message_type;
                self.text_buffer.set_text(text.as_str());
                self.hidden = false;
            }
        }
    }
}
