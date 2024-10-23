#[derive(Debug)]
pub struct VideoOffsetsComponentInit {
    pub audio_video_offset: i64,
    pub subtitle_video_offset: i64,
}

#[derive(Debug)]
pub struct VideoOffsetsComponentModel {
    pub(super) audio_video_offset: i64,
    pub(super) subtitle_video_offset: i64,
}
