use super::about;
use log::*;
use mxl_player_components::{
    actions::{self, Accelerators},
    gst_play::PlayMediaInfo,
    ui::{
        player::{
            messages::{PlaybackState, PlayerComponentInput, PlayerComponentOutput},
            model::{PlayerComponentInit, PlayerComponentModel},
        },
        playlist::{
            messages::{PlaylistChange, PlaylistComponentInput, PlaylistComponentOutput, PlaylistState},
            model::{PlaylistComponentInit, PlaylistComponentModel},
        },
    },
};
use mxl_relm4_components::relm4::{self, actions::*, adw::prelude::*, gtk::glib, prelude::*};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use glib::clone;

type ErrorChannel = Arc<Mutex<Option<anyhow::Error>>>;

pub struct AppInit {
    pub uris: Vec<PathBuf>,
    pub quit_on_stopped: bool,
    pub error_channel: ErrorChannel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppState {
    Next,
    Previous,
    Stopped,
    Paused,
    Playing,
    Buffering,
    Error,
}

pub struct App {
    error_channel: ErrorChannel,
    request_exit: bool,
    ready_to_exit: Arc<Mutex<bool>>,
    current_position: f64,
    duration: f64,
    volume: f64,
    speed: f64,
    app_state: AppState,
    auto_start_done: bool,
    reload_player_on_stopped: bool,
    playlist_component: Controller<PlaylistComponentModel>,
    player_component: Controller<PlayerComponentModel>,
    update_actions: Vec<Box<dyn Fn(AppState)>>,
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum AppMsg {
    PlayerInitialized,
    TogglePlayPause,
    DumpPipeline,
    Stop,
    Stopped,
    Seek(f64),
    NextFrame,
    IncreaseVolume,
    DecreaseVolume,
    ResetVolume,
    SetVolume(f64),
    ChangeVolume(f64),
    IncreaseSpeed,
    DecreaseSpeed,
    ResetSpeed,
    SetSpeed(f64),
    ChangeSpeed(f64),
    SwitchUri(String),
    Previous,
    Next,
    PlayerMediaInfoUpdated(PlayMediaInfo),
    Quit,
    PlaybackError(anyhow::Error),
    DoAutoStart,
}

#[derive(Debug)]
pub enum AppCmd {
    PlayerInitialized(Option<anyhow::Error>),
    PlayerMediaInfoUpdated(PlayMediaInfo),
    PlayerDurationChanged(f64),
    PlayerPositionUpdated(f64),
    PlayerSeekDone,
    PlayerEndOfStream(String),
    PlayerStateChanged(Option<PlaybackState>, PlaybackState),
    PlayerVolumeChanged(f64),
    PlayerSpeedChanged(f64),
    PlayerAudioVideoOffsetChanged(i64),
    PlayerSubtitleVideoOffsetChanged(i64),
    PlayerWarning(anyhow::Error),
    PlayerError(anyhow::Error),
    PlaylistChanged(PlaylistChange),
    PlaylistSwitchUri(String),
    PlaylistEndOfPlaylist,
    PlaylistStateChanged(PlaylistState),
    PlaylistFileChooserRequest,
}

relm4::new_action_group!(WindowActionGroup, "win");
relm4::new_stateless_action!(TogglePlayPause, WindowActionGroup, "toggle-play-pause");
relm4::new_stateless_action!(NextFrame, WindowActionGroup, "next-frame");
relm4::new_stateless_action!(Stop, WindowActionGroup, "stop");
relm4::new_stateless_action!(NextUri, WindowActionGroup, "next-uri");
relm4::new_stateless_action!(PrevUri, WindowActionGroup, "prev-uri");
relm4::new_stateless_action!(IncreaseVolume, WindowActionGroup, "increase-volume");
relm4::new_stateless_action!(DecreaseVolume, WindowActionGroup, "decrease-volume");
relm4::new_stateless_action!(ResetVolume, WindowActionGroup, "reset-volume");
relm4::new_stateless_action!(IncreaseSpeed, WindowActionGroup, "increase-speed");
relm4::new_stateless_action!(DecreaseSpeed, WindowActionGroup, "decrease-speed");
relm4::new_stateless_action!(ResetSpeed, WindowActionGroup, "reset-speed");
relm4::new_stateless_action!(DumpPipeline, WindowActionGroup, "dump-pipeline");

const VOLUME_DEFAULT: f64 = 1.0;
const VOLUME_MIN: f64 = 0.0;
const VOLUME_MAX: f64 = 1.0;
const VOLUME_INCREASE: f64 = 0.1;
const VOLUME_DECREASE: f64 = -VOLUME_INCREASE;

const SPEED_DEFAULT: f64 = 1.0;
const SPEED_MIN: f64 = 0.2;
const SPEED_MAX: f64 = 10.0;
const SPEED_INCREASE: f64 = 0.2;
const SPEED_DECREASE: f64 = -SPEED_INCREASE;

#[allow(deprecated)]
#[relm4::component(pub)]
impl Component for App {
    type Init = AppInit;
    type Input = AppMsg;
    type Output = ();
    type CommandOutput = AppCmd;

    view! {
        #[local]
        app -> adw::Application {},

        #[root]
        #[name = "main_window"]
        adw::ApplicationWindow {
            set_default_size: (1920, 1024),
            #[watch]
            set_title: Some(about::APP_ID),

            #[wrap(Some)]
            set_content: main_box = &gtk::Box {
                model.playlist_component.widget(),

                model.player_component.widget(),
            },
        }
    }

    // Initialize the component.
    fn init(app_init: Self::Init, root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let app = relm4::main_adw_application();

        let playlist_component = PlaylistComponentModel::builder()
            .launch(PlaylistComponentInit { uris: app_init.uris })
            .forward(sender.command_sender(), |msg| match msg {
                PlaylistComponentOutput::PlaylistChanged(x) => AppCmd::PlaylistChanged(x),
                PlaylistComponentOutput::SwitchUri(x) => AppCmd::PlaylistSwitchUri(x),
                PlaylistComponentOutput::EndOfPlaylist => AppCmd::PlaylistEndOfPlaylist,
                PlaylistComponentOutput::StateChanged(state) => AppCmd::PlaylistStateChanged(state),
                PlaylistComponentOutput::FileChooserRequest => AppCmd::PlaylistFileChooserRequest,
            });

        let player_component = {
            PlayerComponentModel::builder()
                .launch(PlayerComponentInit {
                    show_seeking_overlay: false,
                    seek_accurate: false,
                    compositor: None,
                    draw_callback: Box::new(|_, _| {}),
                    drag_gesture: None,
                    motion_tracker: None,
                })
                .forward(sender.command_sender(), |msg| match msg {
                    PlayerComponentOutput::PlayerInitialized(x) => AppCmd::PlayerInitialized(x),
                    PlayerComponentOutput::MediaInfoUpdated(x) => AppCmd::PlayerMediaInfoUpdated(x),
                    PlayerComponentOutput::DurationChanged(x) => AppCmd::PlayerDurationChanged(x),
                    PlayerComponentOutput::PositionUpdated(x) => AppCmd::PlayerPositionUpdated(x),
                    PlayerComponentOutput::SeekDone => AppCmd::PlayerSeekDone,
                    PlayerComponentOutput::EndOfStream(x) => AppCmd::PlayerEndOfStream(x),
                    PlayerComponentOutput::StateChanged(x, y) => AppCmd::PlayerStateChanged(x, y),
                    PlayerComponentOutput::VolumeChanged(x) => AppCmd::PlayerVolumeChanged(x),
                    PlayerComponentOutput::SpeedChanged(x) => AppCmd::PlayerSpeedChanged(x),
                    PlayerComponentOutput::AudioVideoOffsetChanged(x) => AppCmd::PlayerAudioVideoOffsetChanged(x),
                    PlayerComponentOutput::SubtitleVideoOffsetChanged(x) => AppCmd::PlayerSubtitleVideoOffsetChanged(x),
                    PlayerComponentOutput::Warning(x) => AppCmd::PlayerWarning(x),
                    PlayerComponentOutput::Error(x) => AppCmd::PlayerError(x),
                })
        };

        let mut model = App {
            error_channel: app_init.error_channel,
            request_exit: app_init.quit_on_stopped,
            ready_to_exit: Arc::new(Mutex::new(false)),
            current_position: 0.0,
            duration: 0.0,
            volume: VOLUME_DEFAULT,
            speed: SPEED_DEFAULT,
            app_state: AppState::Stopped,
            auto_start_done: false,
            reload_player_on_stopped: false,
            playlist_component,
            player_component,
            update_actions: Vec::new(),
        };

        {
            app.set_accelerators_for_action::<TogglePlayPause>(&actions::accelerators(Accelerators::TogglePlayPause));
            app.set_accelerators_for_action::<NextFrame>(&actions::accelerators(Accelerators::NextFrame));
            app.set_accelerators_for_action::<NextUri>(&actions::accelerators(Accelerators::Next));
            app.set_accelerators_for_action::<PrevUri>(&actions::accelerators(Accelerators::Previous));
            app.set_accelerators_for_action::<IncreaseVolume>(&actions::accelerators(Accelerators::IncreaseVolume));
            app.set_accelerators_for_action::<DecreaseVolume>(&actions::accelerators(Accelerators::DecreaseVolume));
            app.set_accelerators_for_action::<IncreaseSpeed>(&actions::accelerators(Accelerators::IncreaseSpeed));
            app.set_accelerators_for_action::<DecreaseSpeed>(&actions::accelerators(Accelerators::DecreaseSpeed));
            app.set_accelerators_for_action::<ResetSpeed>(&actions::accelerators(Accelerators::ResetSpeed));
            app.set_accelerators_for_action::<DumpPipeline>(&actions::accelerators(Accelerators::DumpPipeline));
        }

        // Generate the widgets based on the view! macro here
        let widgets = view_output!();

        {
            let mut action_group = RelmActionGroup::<WindowActionGroup>::new();
            {
                let action = RelmAction::<TogglePlayPause>::new_stateless(clone!(
                    #[strong]
                    sender,
                    move |_| sender.input(AppMsg::TogglePlayPause)
                ));
                model.update_actions.push(Box::new(clone!(
                    #[strong(rename_to = gio_action)]
                    action.gio_action(),
                    move |app_state| {
                        gio_action.set_enabled(app_state != AppState::Error);
                    }
                )));
                action_group.add_action(action);
            }
            {
                let action = RelmAction::<NextFrame>::new_stateless(clone!(
                    #[strong]
                    sender,
                    move |_| sender.input(AppMsg::NextFrame)
                ));
                model.update_actions.push(Box::new(clone!(
                    #[strong(rename_to = gio_action)]
                    action.gio_action(),
                    move |app_state| {
                        gio_action.set_enabled(app_state == AppState::Paused || app_state == AppState::Playing);
                    }
                )));
                action_group.add_action(action);
            }
            {
                let action = RelmAction::<Stop>::new_stateless(clone!(
                    #[strong]
                    sender,
                    move |_| sender.input(AppMsg::Stop)
                ));
                model.update_actions.push(Box::new(clone!(
                    #[strong(rename_to = gio_action)]
                    action.gio_action(),
                    move |app_state| {
                        gio_action.set_enabled(app_state != AppState::Stopped);
                    }
                )));
                action_group.add_action(action);
            }
            {
                let action = RelmAction::<NextUri>::new_stateless(clone!(
                    #[strong]
                    sender,
                    move |_| sender.input(AppMsg::Next)
                ));
                model.update_actions.push(Box::new(clone!(
                    #[strong(rename_to = gio_action)]
                    action.gio_action(),
                    move |app_state| {
                        gio_action.set_enabled(
                            app_state == AppState::Paused
                                || app_state == AppState::Playing
                                || app_state == AppState::Error,
                        );
                    }
                )));
                action_group.add_action(action);
            }
            {
                let action = RelmAction::<PrevUri>::new_stateless(clone!(
                    #[strong]
                    sender,
                    move |_| sender.input(AppMsg::Previous)
                ));
                model.update_actions.push(Box::new(clone!(
                    #[strong(rename_to = gio_action)]
                    action.gio_action(),
                    move |app_state| {
                        gio_action.set_enabled(
                            app_state == AppState::Paused
                                || app_state == AppState::Playing
                                || app_state == AppState::Error,
                        );
                    }
                )));
                action_group.add_action(action);
            }
            action_group.add_action(RelmAction::<IncreaseVolume>::new_stateless(clone!(
                #[strong]
                sender,
                move |_| sender.input(AppMsg::IncreaseVolume)
            )));
            action_group.add_action(RelmAction::<DecreaseVolume>::new_stateless(clone!(
                #[strong]
                sender,
                move |_| sender.input(AppMsg::DecreaseVolume)
            )));
            action_group.add_action(RelmAction::<ResetVolume>::new_stateless({
                let sender = sender.clone();
                move |_| sender.input(AppMsg::ResetVolume)
            }));
            {
                let action = RelmAction::<IncreaseSpeed>::new_stateless(clone!(
                    #[strong]
                    sender,
                    move |_| sender.input(AppMsg::IncreaseSpeed)
                ));
                model.update_actions.push(Box::new(clone!(
                    #[strong(rename_to = gio_action)]
                    action.gio_action(),
                    move |app_state| {
                        gio_action.set_enabled(app_state != AppState::Stopped);
                    }
                )));
                action_group.add_action(action);
            }
            {
                let action = RelmAction::<DecreaseSpeed>::new_stateless(clone!(
                    #[strong]
                    sender,
                    move |_| sender.input(AppMsg::DecreaseSpeed)
                ));
                model.update_actions.push(Box::new(clone!(
                    #[strong(rename_to = gio_action)]
                    action.gio_action(),
                    move |app_state| {
                        gio_action.set_enabled(app_state != AppState::Stopped);
                    }
                )));
                action_group.add_action(action);
            }
            {
                let action = RelmAction::<ResetSpeed>::new_stateless(clone!(
                    #[strong]
                    sender,
                    move |_| sender.input(AppMsg::ResetSpeed)
                ));
                model.update_actions.push(Box::new(clone!(
                    #[strong(rename_to = gio_action)]
                    action.gio_action(),
                    move |app_state| {
                        gio_action.set_enabled(app_state != AppState::Stopped);
                    }
                )));
                action_group.add_action(action);
            }
            {
                let action = RelmAction::<DumpPipeline>::new_stateless(clone!(
                    #[strong]
                    sender,
                    move |_| sender.input(AppMsg::DumpPipeline)
                ));
                model.update_actions.push(Box::new(clone!(
                    #[strong(rename_to = gio_action)]
                    action.gio_action(),
                    move |app_state| {
                        gio_action.set_enabled(app_state != AppState::Stopped);
                    }
                )));
                action_group.add_action(action);
            }
            action_group.register_for_widget(&widgets.main_window);
        }

        widgets.main_window.connect_close_request(clone!(
            #[weak(rename_to = ready_to_exit)]
            model.ready_to_exit,
            #[upgrade_or]
            gtk::glib::Propagation::Proceed,
            move |_| {
                if !(*ready_to_exit.lock().unwrap()) {
                    sender.input(AppMsg::Quit);
                    gtk::glib::Propagation::Stop
                } else {
                    gtk::glib::Propagation::Proceed
                }
            }
        ));

        ComponentParts { model, widgets }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        msg: Self::Input,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            AppMsg::PlayerInitialized => sender.input(AppMsg::DoAutoStart),
            AppMsg::TogglePlayPause => {
                debug!("Play/pause");
                match self.app_state {
                    AppState::Stopped => {
                        self.playlist_component
                            .sender()
                            .send(PlaylistComponentInput::Start)
                            .unwrap_or_default();
                    }
                    AppState::Playing => {
                        self.player_component
                            .sender()
                            .send(PlayerComponentInput::ChangeState(PlaybackState::Paused))
                            .unwrap_or_default();
                    }
                    AppState::Paused => {
                        self.player_component
                            .sender()
                            .send(PlayerComponentInput::ChangeState(PlaybackState::Playing))
                            .unwrap_or_default();
                    }
                    AppState::Buffering => {
                        self.player_component
                            .sender()
                            .send(PlayerComponentInput::ChangeState(PlaybackState::Paused))
                            .unwrap_or_default();
                    }
                    AppState::Next => (),
                    AppState::Previous => (),
                    AppState::Error => (),
                }
            }
            AppMsg::NextFrame => {
                // Stepping to the next frame is only allowed while in pause:
                if self.app_state != AppState::Paused {
                    sender.input(AppMsg::TogglePlayPause);
                } else {
                    self.player_component
                        .sender()
                        .send(PlayerComponentInput::NextFrame)
                        .unwrap_or_default();
                }
            }
            AppMsg::Stop => {
                self.playlist_component
                    .sender()
                    .send(PlaylistComponentInput::Stop)
                    .unwrap_or_default();
            }
            AppMsg::Stopped => {
                if self.request_exit {
                    sender.input(AppMsg::Quit);
                }
                if self.reload_player_on_stopped {
                    self.reload_player_on_stopped = false;
                    self.player_component.sender().emit(PlayerComponentInput::ReloadPlayer);
                }
            }
            AppMsg::SwitchUri(uri) => {
                sender.input(AppMsg::ResetSpeed);
                self.player_component
                    .sender()
                    .send(PlayerComponentInput::UpdateUri(uri))
                    .unwrap_or_default();
                self.player_component
                    .sender()
                    .send(PlayerComponentInput::ChangeState(PlaybackState::Playing))
                    .unwrap_or_default();
            }
            AppMsg::Previous => {
                trace!("Switch to next previous");
                self.app_state = AppState::Previous;
                self.playlist_component
                    .sender()
                    .send(PlaylistComponentInput::Previous)
                    .unwrap_or_default();
            }
            AppMsg::Next => {
                trace!("Switch to next file");
                self.app_state = AppState::Next;
                self.playlist_component
                    .sender()
                    .send(PlaylistComponentInput::Next)
                    .unwrap_or_default();
            }
            AppMsg::Seek(to) => match self.app_state {
                AppState::Stopped => (),
                _ => {
                    self.current_position = to;
                    self.player_component
                        .sender()
                        .send(PlayerComponentInput::Seek(to))
                        .unwrap_or_default();
                }
            },
            AppMsg::IncreaseVolume => {
                sender.input(AppMsg::ChangeVolume(App::clamp_volume(self.volume + VOLUME_INCREASE)));
            }
            AppMsg::DecreaseVolume => {
                sender.input(AppMsg::ChangeVolume(App::clamp_volume(self.volume + VOLUME_DECREASE)));
            }
            AppMsg::ResetVolume => {
                sender.input(AppMsg::ChangeVolume(VOLUME_DEFAULT));
            }
            AppMsg::SetVolume(vol) => {
                trace!("volume was set to {vol}");
                self.volume = vol;
            }
            AppMsg::ChangeVolume(vol) => {
                if self.volume != vol {
                    trace!("change volume to {vol}");
                    self.player_component
                        .sender()
                        .send(PlayerComponentInput::SetVolume(vol))
                        .unwrap_or_default();
                }
            }
            AppMsg::IncreaseSpeed => {
                sender.input(AppMsg::ChangeSpeed(App::clamp_speed(self.speed + SPEED_INCREASE)));
            }
            AppMsg::DecreaseSpeed => {
                sender.input(AppMsg::ChangeSpeed(App::clamp_speed(self.speed + SPEED_DECREASE)));
            }
            AppMsg::ResetSpeed => {
                sender.input(AppMsg::ChangeSpeed(SPEED_DEFAULT));
            }
            AppMsg::SetSpeed(speed) => {
                trace!("speed was set to {speed}");
                self.speed = speed;
            }
            AppMsg::ChangeSpeed(speed) => {
                if self.speed != speed {
                    trace!("change speed to {speed}");
                    self.player_component
                        .sender()
                        .send(PlayerComponentInput::SetSpeed(speed))
                        .unwrap_or_default();
                }
            }
            AppMsg::PlayerMediaInfoUpdated(_) => (),
            AppMsg::DumpPipeline => {
                debug!("Dump pipeline");
                self.player_component
                    .sender()
                    .send(PlayerComponentInput::DumpPipeline(
                        chrono::Local::now().format("mxl_player_%Y-%m-%d_%H_%M_%S").to_string(),
                    ))
                    .unwrap_or_default();
                debug!("Dumped pipeline");
            }
            AppMsg::Quit => {
                if self.app_state != AppState::Stopped {
                    self.request_exit = true;
                    sender.input(AppMsg::Stop);
                } else {
                    {
                        let mut rte = self.ready_to_exit.lock().unwrap();
                        *rte = true;
                    }
                    widgets.main_window.close();
                }
            }
            AppMsg::PlaybackError(error) => {
                error!("{}", error);
                let mut data = self.error_channel.lock().unwrap();
                data.replace(error);
                sender.input(AppMsg::Quit);
            }
            AppMsg::DoAutoStart => {
                if !self.auto_start_done {
                    self.auto_start_done = true;
                    if !self.playlist_component.model().uris.is_empty() {
                        sender.input(AppMsg::TogglePlayPause);
                    }
                }
            }
        }
        self.update_actions();
        self.update_view(widgets, sender)
    }

    fn update_cmd(&mut self, msg: Self::CommandOutput, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            AppCmd::PlayerInitialized(error) => {
                if let Some(error) = error {
                    error!("{error:?}");
                    let mut data = self.error_channel.lock().unwrap();
                    data.replace(error);
                    sender.input(AppMsg::Quit);
                } else {
                    sender.input(AppMsg::PlayerInitialized);
                }
            }
            AppCmd::PlayerMediaInfoUpdated(media_info) => {
                sender.input(AppMsg::PlayerMediaInfoUpdated(media_info));
            }
            AppCmd::PlayerEndOfStream(a) => {
                debug!("player end of stream : {a}");
                sender.input(AppMsg::Next)
            }
            AppCmd::PlayerDurationChanged(duration) => {
                self.duration = duration;
            }
            AppCmd::PlayerPositionUpdated(pos) => {
                self.current_position = pos;
                // debug!("player position updated {pos}");
            }
            AppCmd::PlayerSeekDone => {
                debug!("player seek done");
            }
            AppCmd::PlayerStateChanged(old_state, new_state) => {
                debug!("playback state changed from {old_state:?} to {new_state:?}");
                match new_state {
                    PlaybackState::Stopped => {
                        self.app_state = AppState::Stopped;
                        self.duration = 0.0;
                        self.current_position = 0.0;
                        self.playlist_component
                            .sender()
                            .send(PlaylistComponentInput::PlayerStopped)
                            .unwrap_or_default();
                    }
                    PlaybackState::Playing => {
                        self.app_state = AppState::Playing;
                        self.playlist_component
                            .sender()
                            .send(PlaylistComponentInput::PlayerPlaying)
                            .unwrap_or_default();
                    }
                    PlaybackState::Paused => self.app_state = AppState::Paused,
                    PlaybackState::Buffering => self.app_state = AppState::Buffering,
                    PlaybackState::Error => self.app_state = AppState::Error,
                }
            }
            AppCmd::PlayerVolumeChanged(vol) => sender.input(AppMsg::SetVolume(vol)),
            AppCmd::PlayerSpeedChanged(speed) => sender.input(AppMsg::SetSpeed(speed)),
            AppCmd::PlayerAudioVideoOffsetChanged(offset) => trace!("AppCmd::PlayerAudioVideoOffsetChanged({offset})"),
            AppCmd::PlayerSubtitleVideoOffsetChanged(offset) => {
                trace!("AppCmd::PlayerSubtitleVideoOffsetChanged({offset})")
            }
            AppCmd::PlayerWarning(error) => {
                warn!("Internal player warning: {error:?}");
            }
            AppCmd::PlayerError(error) => {
                error!("Internal player error: {error:?}");
                sender.input(AppMsg::PlaybackError(error));
            }
            AppCmd::PlaylistChanged(change) => {
                debug!("Playlist changed: {change:?}");
                match change {
                    PlaylistChange::Added => trace!("PlaylistChange::Added"),
                    PlaylistChange::Removed => trace!("PlaylistChange::Removed"),
                    PlaylistChange::Updated => trace!("PlaylistChange::Updated"),
                    PlaylistChange::Reordered => trace!("PlaylistChange::Reordered"),
                }
            }
            AppCmd::PlaylistSwitchUri(uri) => sender.input(AppMsg::SwitchUri(uri)),
            AppCmd::PlaylistEndOfPlaylist => {
                info!("End of playlist reached");
                sender.input(AppMsg::Stop);
            }
            AppCmd::PlaylistStateChanged(state) => match state {
                PlaylistState::Playing => (),
                PlaylistState::Stopping => {
                    self.player_component
                        .sender()
                        .send(PlayerComponentInput::ChangeState(PlaybackState::Stopped))
                        .unwrap_or_default();
                }
                PlaylistState::Stopped => {
                    sender.input(AppMsg::Stopped);
                }
            },
            AppCmd::PlaylistFileChooserRequest => (),
        }
        self.update_actions();
    }
}

impl App {
    fn clamp_volume(volume: f64) -> f64 {
        volume.clamp(VOLUME_MIN, VOLUME_MAX)
    }

    fn clamp_speed(speed: f64) -> f64 {
        speed.clamp(SPEED_MIN, SPEED_MAX)
    }

    fn update_actions(&self) {
        for update_action in &self.update_actions {
            update_action(self.app_state);
        }
    }
}
