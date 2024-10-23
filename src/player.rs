use anyhow::{Context, Result};
use gst::{event::Step, format::Buffers, glib, prelude::*};
use gst_play::PlayMessage;
use log::*;
use mxl_relm4_components::relm4::{self, gtk::gdk, Sender};
use std::sync::{Arc, Mutex};

use glib::clone;

use crate::ui::player::messages::{PlaybackState, PlayerComponentCommand, Track};

const GLSINKBIN_NAME: &str = "glsinkbin";

#[derive(Debug)]
pub struct PlayerBuilder {
    seek_accurate: bool,
    compositor: Option<gst::Element>,
    audio_offset: i64,
    subtitle_offset: i64,
}

impl Default for PlayerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl PlayerBuilder {
    pub fn new() -> Self {
        Self {
            seek_accurate: false,
            compositor: None,
            audio_offset: 0,
            subtitle_offset: 0,
        }
    }

    pub fn seek_accurate(&mut self, seek_accurate: bool) -> &mut Self {
        self.seek_accurate = seek_accurate;
        self
    }

    pub fn compositor(&mut self, compositor: Option<gst::Element>) -> &mut Self {
        self.compositor = compositor;
        self
    }

    pub fn audio_offset(&mut self, offset: i64) -> &mut Self {
        self.audio_offset = offset;
        self
    }

    pub fn subtitle_offset(&mut self, offset: i64) -> &mut Self {
        self.subtitle_offset = offset;
        self
    }

    pub fn build(&self, sender: relm4::Sender<PlayerComponentCommand>) -> Result<Player> {
        let gtk_sink = gst::ElementFactory::make("gtk4paintablesink").build()?;

        let paintable = gtk_sink.property::<gdk::Paintable>("paintable");
        paintable.set_property("force-aspect-ratio", true);
        paintable.set_property("use-scaling-filter", true);

        let video_sink = if paintable.property::<Option<gdk::GLContext>>("gl-context").is_some()
            && gst::ElementFactory::find(GLSINKBIN_NAME).is_some()
        {
            debug!("Use GL rendering for playback view");
            gst::ElementFactory::make(GLSINKBIN_NAME)
                .property("sink", &gtk_sink)
                .build()
                .with_context(|| "Failed to create player with element to process GL textures")?
        } else {
            warn!("Use software rendering for playback view");
            gtk_sink.clone()
        };

        let renderer = gst_play::PlayVideoOverlayVideoRenderer::with_sink(&video_sink);

        let gst_play = gst_play::Play::new(Some(renderer.clone().upcast::<gst_play::PlayVideoRenderer>()));

        let pipeline = gst_play.pipeline();
        pipeline.set_property_from_str(
            "flags",
            "soft-colorbalance+deinterlace+buffering+soft-volume+text+audio+video+vis",
        );
        if let Some(compositor) = &self.compositor {
            pipeline.set_property("video-stream-combiner", compositor);
        }

        let mut config = gst_play.config();
        config.set_seek_accurate(self.seek_accurate);
        config.set_position_update_interval(250);
        gst_play
            .set_config(config)
            .with_context(|| "Failed to set player configuration")?;

        let player_data = Arc::new(Mutex::new(PlayerData {
            sender,
            current_state: None,
        }));

        let _bus_watch = gst_play
            .message_bus()
            .add_watch_local(clone!(
                #[weak]
                gst_play,
                #[weak]
                player_data,
                #[upgrade_or]
                glib::ControlFlow::Break,
                move |_, message| {
                    match PlayMessage::parse(message) {
                        Ok(PlayMessage::EndOfStream) => {
                            if let Some(uri) = gst_play.uri() {
                                let player_data = player_data.lock().unwrap();
                                player_data.send(PlayerComponentCommand::EndOfStream(uri.into()));
                            }
                        }
                        Ok(PlayMessage::MediaInfoUpdated { info }) => {
                            let mut player_data_guard = player_data.as_ref().lock();
                            let player_data = player_data_guard.as_mut().unwrap();
                            player_data.send(PlayerComponentCommand::MediaInfoUpdated(info));
                        }
                        Ok(PlayMessage::DurationChanged { duration }) => {
                            let player_data = player_data.lock().unwrap();
                            if let Some(duration) = duration {
                                player_data.send(PlayerComponentCommand::DurationChanged(
                                    duration.mseconds() as f64 / 1000_f64,
                                ));
                            }
                        }
                        Ok(PlayMessage::PositionUpdated { position }) => {
                            let player_data = player_data.lock().unwrap();
                            if let Some(position) = position {
                                player_data.send(PlayerComponentCommand::PositionUpdated(
                                    position.mseconds() as f64 / 1000_f64,
                                ));
                            }
                        }
                        Ok(PlayMessage::VideoDimensionsChanged { width, height }) => {
                            let player_data = player_data.lock().unwrap();
                            player_data.send(PlayerComponentCommand::VideoDimensionsChanged(
                                width as i32,
                                height as i32,
                            ));
                        }
                        Ok(PlayMessage::StateChanged { state }) => {
                            let state = match state {
                                gst_play::PlayState::Playing => Some(PlaybackState::Playing),
                                gst_play::PlayState::Paused => Some(PlaybackState::Paused),
                                gst_play::PlayState::Stopped => Some(PlaybackState::Stopped),
                                gst_play::PlayState::Buffering => Some(PlaybackState::Buffering),
                                _ => None,
                            };
                            if let Some(s) = state {
                                let mut player_data = player_data.lock().unwrap();
                                player_data.change_state(s);
                            }
                        }
                        Ok(PlayMessage::VolumeChanged { volume }) => {
                            let player_data = player_data.lock().unwrap();
                            player_data.send(PlayerComponentCommand::VolumeChanged(volume));
                        }
                        Ok(PlayMessage::Error { error, .. }) => {
                            let mut player_data = player_data.lock().unwrap();
                            player_data.change_state(PlaybackState::Error);
                            player_data.send(PlayerComponentCommand::Error(anyhow::anyhow!(error)));
                        }
                        Ok(PlayMessage::SeekDone) => {
                            let player_data = player_data.lock().unwrap();
                            player_data.send(PlayerComponentCommand::SeekDone);
                        }
                        Ok(PlayMessage::Warning { error, .. }) => {
                            let player_data = player_data.lock().unwrap();
                            player_data.send(PlayerComponentCommand::Warning(anyhow::anyhow!(error)));
                        }
                        _ => (),
                    }

                    glib::ControlFlow::Continue
                }
            ))
            .with_context(|| "Cannot add watcher to player bus")?;

        gst_play.connect_audio_video_offset_notify(clone!(
            #[weak]
            player_data,
            move |play| {
                let player_data = player_data.lock().unwrap();
                player_data.send(PlayerComponentCommand::AudioVideoOffsetChanged(
                    play.audio_video_offset(),
                ));
            }
        ));

        gst_play.connect_subtitle_video_offset_notify(clone!(
            #[weak]
            player_data,
            move |play| {
                let player_data = player_data.lock().unwrap();
                player_data.send(PlayerComponentCommand::SubtitleVideoOffsetChanged(
                    play.subtitle_video_offset(),
                ));
            }
        ));

        let player = Player {
            player: gst_play,
            renderer,
            gtk_sink,
            _bus_watch,
            data: player_data,
        };

        player.set_audio_video_offset(self.audio_offset);
        player.set_subtitle_video_offset(self.subtitle_offset);

        Ok(player)
    }
}

#[derive(Debug)]
pub struct Player {
    player: gst_play::Play,
    renderer: gst_play::PlayVideoOverlayVideoRenderer,
    gtk_sink: gst::Element,
    _bus_watch: gst::bus::BusWatchGuard,
    data: Arc<Mutex<PlayerData>>,
}

#[derive(Debug)]
struct PlayerData {
    sender: Sender<PlayerComponentCommand>,
    current_state: Option<PlaybackState>,
}

impl PlayerData {
    fn change_state(&mut self, new_state: PlaybackState) {
        let target_state = if let Some(current_state) = self.current_state {
            if current_state != new_state {
                if current_state == PlaybackState::Error && new_state == PlaybackState::Stopped {
                    // Do not change from Error state to Stopped, because we want to stay in the Error state until the user takes action:
                    trace!(
                        "Ignore player state change from {:?} to {:?}",
                        PlaybackState::Error,
                        PlaybackState::Stopped,
                    );
                    None
                } else {
                    Some(new_state)
                }
            } else {
                None
            }
        } else {
            Some(new_state)
        };
        if let Some(target_state) = target_state {
            self.set_state(target_state);
        }
    }

    fn set_state(&mut self, new_state: PlaybackState) {
        let old_state = self.current_state;
        self.current_state = Some(new_state);
        trace!("player state changed from {old_state:?} to {new_state:?}");
        self.send(PlayerComponentCommand::StateChanged(old_state, new_state));
    }

    fn send(&self, cmd: PlayerComponentCommand) {
        self.sender.send(cmd).unwrap_or_default();
    }
}

impl Player {
    pub fn paintable(&self) -> gdk::Paintable {
        self.gtk_sink.property::<gdk::Paintable>("paintable")
    }

    pub fn update_render_rectangle(&self, src_rect: &gst_video::VideoRectangle, new_rect: gst_video::VideoRectangle) {
        let rect = gst_video::center_video_rectangle(src_rect, &new_rect, true);
        self.renderer.set_render_rectangle(rect.x, rect.y, rect.w, rect.h);
        self.renderer.expose();
    }

    pub fn set_uri(&self, uri: &str) {
        debug!("player set uri {uri}");
        self.player.set_uri(Some(uri));
    }

    pub fn play(&self) {
        self.player.play();
    }

    pub fn pause(&self) {
        self.player.pause();
    }

    pub fn stop(&self) {
        let mut player_data = self.data.lock().unwrap();
        if let Some(current_state) = player_data.current_state {
            match current_state {
                PlaybackState::Stopped => player_data.set_state(PlaybackState::Stopped),
                PlaybackState::Playing => (),
                PlaybackState::Paused => (),
                PlaybackState::Buffering => (),
                PlaybackState::Error => {
                    // Force state change from Error to Stopped, because the player implicitly was stopped by the previous error:
                    trace!(
                        "Explicitly change player state from {:?} to {:?}",
                        PlaybackState::Error,
                        PlaybackState::Stopped,
                    );
                    player_data.set_state(PlaybackState::Stopped)
                }
            }
        }
        drop(player_data);
        self.player.stop();
    }

    pub fn seek(&self, to: &f64) {
        let to = gst::ClockTime::from_mseconds((to * 1000_f64) as u64);
        self.player.seek(to);
    }

    pub fn set_volume(&self, vol: f64) {
        self.player.set_volume(vol);
    }

    pub fn set_audio_track(&self, track: Track) -> Result<()> {
        match track {
            Track::Enable => self.player.set_audio_track_enabled(true),
            Track::Disable => self.player.set_audio_track_enabled(false),
            Track::Stream(index) => {
                self.player.set_audio_track_enabled(true);
                self.player
                    .set_audio_track(index)
                    .with_context(|| "Cannot set audio stream")?
            }
        }
        Ok(())
    }

    pub fn speed(&self) -> f64 {
        self.player.rate()
    }

    pub fn set_speed(&self, speed: f64) {
        self.player.set_rate(speed);
    }

    pub fn next_frame(&self) {
        trace!("step to next frame");
        self.player
            .pipeline()
            .send_event(Step::new(Buffers::from_u64(1), 1., true, false));
    }

    pub fn set_audio_video_offset(&self, offset: i64) {
        self.player.set_audio_video_offset(offset);
    }

    pub fn set_subtitle_video_offset(&self, offset: i64) {
        self.player.set_subtitle_video_offset(offset);
    }

    pub fn dump_pipeline(&self, label: &str) {
        let element = self.player.pipeline();
        if let Ok(pipeline) = element.downcast::<gst::Pipeline>() {
            pipeline.debug_to_dot_file_with_ts(gst::DebugGraphDetails::all(), label);
        }
    }
}
