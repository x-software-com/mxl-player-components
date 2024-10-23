use mxl_relm4_components::relm4::prelude::DynamicIndex;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy)]
pub enum PlaylistState {
    Stopped,
    Playing,
    Stopping,
}

#[derive(Debug, Clone, Copy)]
pub enum RepeatMode {
    Off,
    All,
}

#[derive(Debug, Clone, Copy)]
pub enum SortOrder {
    StartTime,
    ShortUri,
}

#[derive(Debug, Clone, Copy)]
pub enum PlaylistChange {
    Added,
    Removed,
    Updated,
    Reordered,
}

#[derive(Debug)]
pub enum PlaylistComponentInput {
    Start,
    Stop,
    Previous,
    Next,
    PlayerPlaying,
    PlayerStopped,
    Activate(usize),
    Switch(DynamicIndex),
    EndOfPlaylist(DynamicIndex),
    Add(Vec<PathBuf>),
    AddBefore(DynamicIndex, Vec<PathBuf>),
    AddAfter(DynamicIndex, Vec<PathBuf>),
    Remove(DynamicIndex),
    Updated(DynamicIndex),
    Move(DynamicIndex, usize),
    FetchMetadata,
    FileChooserRequest,
    Sort(SortOrder),
    ToggleRepeat,
}

#[derive(Debug)]
pub enum PlaylistComponentOutput {
    PlaylistChanged(PlaylistChange),
    SwitchUri(String),
    EndOfPlaylist,
    StateChanged(PlaylistState),
    FileChooserRequest,
}

#[derive(Debug)]
pub enum PlaylistCommandOutput {
    ShowPlaceholder(bool),
}
