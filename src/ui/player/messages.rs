use gst_play::PlayMediaInfo;

#[derive(Debug)]
pub enum Track {
    Enable,
    Disable,
    Stream(i32),
}

#[derive(Debug)]
pub enum PlayerComponentInput {
    UpdateUri(String),
    ChangeState(PlaybackState),
    SwitchAudioTrack(Track),
    Seek(f64),
    NextFrame,
    SetVolume(f64),
    SetSpeed(f64),
    DumpPipeline(String),
    SetZoomRelative(f64),
    SetZoom(Option<f64>),
    SetAudioVideoOffset(i64),
    SetSubtitleVideoOffset(i64),
    SetOverlayVisible(bool),
    RequestOverlayRedraw,
    ReloadPlayer,
    PrivateMessage(internal::PrivateMsg),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackState {
    Stopped,
    Paused,
    Playing,
    Buffering,
    Error,
}

#[derive(Debug)]
pub enum PlayerComponentOutput {
    PlayerInitialized(Option<anyhow::Error>),
    MediaInfoUpdated(PlayMediaInfo),
    DurationChanged(f64),
    PositionUpdated(f64),
    SeekDone,
    EndOfStream(std::string::String),
    StateChanged(Option<PlaybackState>, PlaybackState),
    VolumeChanged(f64),
    SpeedChanged(f64),
    AudioVideoOffsetChanged(i64),
    SubtitleVideoOffsetChanged(i64),
    Warning(anyhow::Error),
    Error(anyhow::Error),
}

#[derive(Debug)]
pub enum PlayerComponentCommand {
    MediaInfoUpdated(PlayMediaInfo),
    PositionUpdated(f64),
    DurationChanged(f64),
    SeekDone,
    EndOfStream(std::string::String),
    StateChanged(Option<PlaybackState>, PlaybackState),
    VideoDimensionsChanged(i32, i32),
    VolumeChanged(f64),
    AudioVideoOffsetChanged(i64),
    SubtitleVideoOffsetChanged(i64),
    Warning(anyhow::Error),
    Error(anyhow::Error),
}

pub(super) mod internal {
    #[derive(Debug)]
    pub enum PrivateMsg {
        DragBegin(f64, f64),
        DragUpdate(f64, f64),
        DragEnd(f64, f64),
        MotionDetected(f64, f64),
    }
}
