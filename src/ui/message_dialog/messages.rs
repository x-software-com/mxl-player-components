#[derive(Debug, PartialEq)]
pub enum MessageDialogType {
    Fatal,
    Error,
    Warning,
}

#[derive(Debug)]
pub enum MessageDialogInput {
    Message(MessageDialogType, Option<String>, String),
    PrivateMessage(internal::PrivateMsg),
}

#[derive(Debug)]
pub enum MessageDialogOutput {
    CreateReport,
    Quit,
}

pub(super) mod internal {
    #[derive(Debug)]
    pub enum PrivateMsg {
        CreateReportButtonPressed,
        CloseButtonPressed,
    }
}
