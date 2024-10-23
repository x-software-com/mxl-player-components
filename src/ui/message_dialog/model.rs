use super::messages::MessageDialogType;
use mxl_relm4_components::relm4::gtk;

#[derive(Debug)]
pub struct MessageDialog {
    pub(super) hidden: bool,
    pub(super) dialog_type: MessageDialogType,
    pub(super) title: String,
    pub(super) text_buffer: gtk::TextBuffer,
}
