#[derive(Debug)]
pub enum VideoOffsetsComponentInput {
    SetAudioVideoOffset(i64),
    SetSubtitleVideoOffset(i64),
    PrivateMessage(internal::PrivateMsg),
}

#[derive(Debug)]
pub enum VideoOffsetsComponentOutput {
    SetAudioVideoOffset(i64),
    SetSubtitleVideoOffset(i64),
}

pub(super) mod internal {
    #[derive(Debug)]
    pub enum PrivateMsg {
        AudioVideoOffsetChanged(i64),
        SubtitleVideoOffsetChanged(i64),
    }
}
