use gst_video::VideoRectangle;
use log::*;
use mxl_relm4_components::relm4::{self, gtk::glib, gtk::prelude::*, prelude::*};
use std::{borrow::BorrowMut, rc::Rc, sync::Mutex};

use glib::clone;

use super::{
    messages::{
        internal::PrivateMsg, PlaybackState, PlayerComponentCommand, PlayerComponentInput, PlayerComponentOutput,
    },
    model::{PlayerComponentInit, PlayerComponentModel, ViewData},
};
use crate::player::PlayerBuilder;
use crate::{localization::helper::fl, ui::player::model::DrawCallbackData};

const SCALE_MULTIPLIER: f64 = 2.0;

#[relm4::component(pub)]
impl Component for PlayerComponentModel {
    type Init = PlayerComponentInit;
    type Input = PlayerComponentInput;
    type Output = PlayerComponentOutput;
    type CommandOutput = PlayerComponentCommand;

    view! {
        #[name = "video_view"]
        gtk::Overlay {
            #[name = "video_scrolled_window"]
            gtk::ScrolledWindow {
                // Set scrollbar policy to external, to disable them (Never disables scrolling at all):
                set_hscrollbar_policy: gtk::PolicyType::External,
                set_vscrollbar_policy: gtk::PolicyType::External,
                set_vexpand: true,
                set_hexpand: true,


                gtk::Overlay {
                    #[name = "video_picture"]
                    gtk::Picture {
                        set_content_fit: gtk::ContentFit::Fill,
                    },


                    add_overlay = drawing_overlay = &gtk::DrawingArea {
                        #[watch]
                        set_visible: model.show_drawing_overlay && model.playback_state != PlaybackState::Stopped && model.playback_state != PlaybackState::Error,
                        set_vexpand: true,
                        set_hexpand: true,
                        set_can_target: true,

                    },
                },
            },

            add_overlay = overlay = &gtk::Box {
                #[watch]
                set_visible: model.show_seeking_overlay && model.playback_state == PlaybackState::Buffering,
                add_css_class: "osd",
                set_vexpand: true,
                set_hexpand: true,
                set_can_target: false,

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_hexpand: true,
                    set_vexpand: true,
                    set_halign: gtk::Align::Center,
                    set_valign: gtk::Align::Center,
                    set_spacing: 8,

                    gtk::Label {
                        #[watch]
                        set_label: if model.seeking {
                            fl!("seeking").clone()
                        } else {
                            fl!("buffering").clone()
                        }.as_ref(),
                        set_css_classes: &["title-4"],
                    },

                    gtk::Spinner {
                        #[watch]
                        set_spinning: overlay.is_visible(),
                        set_size_request: (20, 20),
                    },
                },
            },
        }
    }

    // Initialize the component.
    fn init(init: Self::Init, root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let mut player_builder = PlayerBuilder::new();

        player_builder
            .seek_accurate(init.seek_accurate)
            .compositor(init.compositor);

        let player = match player_builder.build(sender.command_sender().clone()) {
            Ok(player) => {
                sender
                    .output_sender()
                    .send(PlayerComponentOutput::PlayerInitialized(None))
                    .unwrap_or_default();
                Some(player)
            }
            Err(error) => {
                sender
                    .output_sender()
                    .send(PlayerComponentOutput::PlayerInitialized(Some(error)))
                    .unwrap_or_default();
                None
            }
        };

        let model = PlayerComponentModel {
            player_builder,
            player,
            playback_state: PlaybackState::Stopped,
            show_seeking_overlay: init.show_seeking_overlay,
            seeking: false,
            show_drawing_overlay: false,
            view_data: Rc::new(Mutex::new(ViewData::default())),
            draw_callback: Rc::new(Mutex::new(DrawCallbackData::new(init.draw_callback))),
            drag_position: None,
            mouse_position: None,
        };

        // Insert the code generation of the view! macro here
        let widgets = view_output!();

        if let Some(player) = &model.player {
            widgets.video_picture.set_paintable(Some(player.paintable()).as_ref());
        }

        {
            let mut view_data = model.view_data.lock().unwrap();
            view_data.video_view.set_cursor_widgets(vec![
                widgets.video_view.clone().upcast(),
                widgets.drawing_overlay.clone().upcast(),
                widgets.video_picture.clone().upcast(),
            ]);
        }

        widgets.drawing_overlay.set_draw_func(clone!(
            #[weak(rename_to = draw_callback)]
            model.draw_callback,
            #[weak(rename_to = view_data)]
            model.view_data,
            #[strong(rename_to = video_scrolled_window)]
            widgets.video_scrolled_window,
            #[strong(rename_to = video_picture)]
            widgets.video_picture,
            move |_drawing_area, context, w, h| {
                trace!("Drawing func called... w={w} h={h}");
                let mut view_data = view_data.lock().unwrap();
                if view_data.video_view.drawing_area.is_none() {
                    view_data.video_view.drawing_area = Some(VideoRectangle::new(0, 0, w, h));
                } else if let Some(drawing_area) = &mut view_data.video_view.drawing_area {
                    if drawing_area.w != w || drawing_area.h != h {
                        drawing_area.w = w;
                        drawing_area.h = h;
                    }
                }
                view_data
                    .video_view
                    .update(None, &video_scrolled_window, &video_picture);
                let draw_callback = draw_callback.lock().unwrap();
                (draw_callback.draw_callback)(context, view_data.video_view.borrow_mut());
            }
        ));

        if let Some(drag_gesture) = init.drag_gesture {
            widgets.drawing_overlay.add_controller(drag_gesture);
        }

        if let Some(motion_tracker) = init.motion_tracker {
            widgets.drawing_overlay.add_controller(motion_tracker);
        }

        widgets
            .video_scrolled_window
            .add_controller(model.new_gesture_drag(sender.clone()));

        widgets
            .video_scrolled_window
            .add_controller(model.new_wheel_zoom(sender.clone()));

        widgets
            .video_scrolled_window
            .add_controller(model.new_motion_tracker(sender));

        ComponentParts { model, widgets }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        msg: Self::Input,
        sender: ComponentSender<Self>,
        _: &Self::Root,
    ) {
        if let Some(player) = &self.player {
            match msg {
                PlayerComponentInput::UpdateUri(uri) => {
                    player.set_uri(&uri);
                }
                PlayerComponentInput::ChangeState(state) => match state {
                    PlaybackState::Playing => player.play(),
                    PlaybackState::Paused => player.pause(),
                    PlaybackState::Stopped => player.stop(),
                    PlaybackState::Buffering => panic!("Cannot explicitly change playback state to buffering"),
                    PlaybackState::Error => panic!("Cannot explicitly change playback state to error"),
                },
                PlayerComponentInput::SwitchAudioTrack(track) => {
                    if let Err(error) = player.set_audio_track(track) {
                        sender.output(PlayerComponentOutput::Error(error)).unwrap_or_default();
                    }
                }
                PlayerComponentInput::Seek(to) => {
                    self.seeking = true;
                    player.seek(&to);
                }
                PlayerComponentInput::NextFrame => {
                    player.next_frame();
                }
                PlayerComponentInput::SetVolume(vol) => {
                    player.set_volume(vol);
                }
                PlayerComponentInput::SetSpeed(speed) => {
                    let current_speed = player.speed();
                    player.set_speed(speed);
                    if current_speed != speed {
                        sender
                            .output(PlayerComponentOutput::SpeedChanged(speed))
                            .unwrap_or_default();
                    }
                }
                PlayerComponentInput::DumpPipeline(label) => {
                    player.dump_pipeline(&label);
                }
                PlayerComponentInput::SetZoomRelative(scale) => {
                    trace!("New zoom: {scale}");
                    let scale = {
                        let view_data = self.view_data.lock().unwrap();
                        view_data.video_view.zoom_factor + scale
                    };
                    self.set_zoom(
                        Some(scale),
                        &mut widgets.video_scrolled_window,
                        &mut widgets.video_picture,
                    );
                    widgets.drawing_overlay.queue_draw();
                }
                PlayerComponentInput::SetZoom(scale) => {
                    self.set_zoom(scale, &mut widgets.video_scrolled_window, &mut widgets.video_picture);
                    widgets.drawing_overlay.queue_draw();
                }
                PlayerComponentInput::SetAudioVideoOffset(offset) => {
                    self.player_builder.audio_offset(offset);
                    player.set_audio_video_offset(offset);
                }
                PlayerComponentInput::SetSubtitleVideoOffset(offset) => {
                    self.player_builder.subtitle_offset(offset);
                    player.set_subtitle_video_offset(offset);
                }
                PlayerComponentInput::SetOverlayVisible(visible) => {
                    self.show_drawing_overlay = visible;
                    widgets.drawing_overlay.queue_draw();
                }
                PlayerComponentInput::RequestOverlayRedraw => widgets.drawing_overlay.queue_draw(),
                PlayerComponentInput::ReloadPlayer => {
                    self.player = match self.player_builder.build(sender.command_sender().clone()) {
                        Ok(player) => {
                            widgets.video_picture.set_paintable(Some(player.paintable()).as_ref());
                            Some(player)
                        }
                        Err(error) => {
                            sender.output_sender().emit(PlayerComponentOutput::Error(error));
                            None
                        }
                    };
                }
                PlayerComponentInput::PrivateMessage(msg) => match msg {
                    PrivateMsg::MotionDetected(x, y) => {
                        self.mouse_position = Some((x, y));
                    }
                    PrivateMsg::DragBegin(_, _) => {
                        // Start the drag position at 0.0, 0.0:
                        self.drag_position = Some((0.0, 0.0));
                        let mut view_data = self.view_data.lock().unwrap();
                        if view_data.video_view.zoom_factor != 1.0 {
                            view_data.video_view.set_cursor(Some("grabbing"));
                        }
                    }
                    PrivateMsg::DragUpdate(x, y) => {
                        if let Some((old_x, old_y)) = self.drag_position {
                            // Calculate offset relative to the last darg_position:
                            let x_offset = old_x - x;
                            let y_offset = old_y - y;

                            let ha = widgets.video_scrolled_window.hadjustment();
                            let va = widgets.video_scrolled_window.vadjustment();
                            // Update scrolled window position:
                            ha.set_value(ha.value() + x_offset);
                            va.set_value(va.value() + y_offset);

                            // Set the current position:
                            self.drag_position = Some((x, y));
                        }
                    }
                    PrivateMsg::DragEnd(_, _) => {
                        // Remove drag position:
                        self.drag_position = None;
                        if widgets.video_picture.cursor().is_some() {
                            let mut view_data = self.view_data.lock().unwrap();
                            if view_data.video_view.zoom_factor != 1.0 {
                                view_data.video_view.set_cursor(Some("grab"));
                            } else {
                                view_data.video_view.set_cursor(None);
                            }
                        }
                    }
                },
            }
        }
        self.update_view(widgets, sender)
    }

    fn update_cmd(&mut self, msg: Self::CommandOutput, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            PlayerComponentCommand::VideoDimensionsChanged(width, height) => {
                if width != 0 && height != 0 {
                    let mut view_data = self.view_data.lock().unwrap();
                    let new_dimensions = Some(gst_video::VideoRectangle::new(0, 0, width, height));
                    if new_dimensions != view_data.video_view.video_dimensions {
                        view_data.video_view.video_dimensions = new_dimensions;
                        debug!("video dimensions changed: {width}x{height}");
                        sender.input(PlayerComponentInput::SetZoom(None));
                    }
                }
            }
            PlayerComponentCommand::MediaInfoUpdated(info) => {
                sender
                    .output(PlayerComponentOutput::MediaInfoUpdated(info))
                    .unwrap_or_default();
            }
            PlayerComponentCommand::DurationChanged(duration) => {
                sender
                    .output(PlayerComponentOutput::DurationChanged(duration))
                    .unwrap_or_default();
            }
            PlayerComponentCommand::PositionUpdated(pos) => {
                sender
                    .output(PlayerComponentOutput::PositionUpdated(pos))
                    .unwrap_or_default();
            }
            PlayerComponentCommand::SeekDone => {
                self.seeking = false;
                sender.output(PlayerComponentOutput::SeekDone).unwrap_or_default();
            }
            PlayerComponentCommand::EndOfStream(val) => {
                sender
                    .output(PlayerComponentOutput::EndOfStream(val))
                    .unwrap_or_default();
            }
            PlayerComponentCommand::StateChanged(old_state, new_state) => {
                self.playback_state = new_state;
                let reset_states = match new_state {
                    PlaybackState::Stopped => true,
                    PlaybackState::Paused => false,
                    PlaybackState::Playing => false,
                    PlaybackState::Buffering => false,
                    PlaybackState::Error => true,
                };
                if reset_states {
                    self.seeking = false;
                }
                sender.input_sender().emit(PlayerComponentInput::RequestOverlayRedraw);
                sender
                    .output(PlayerComponentOutput::StateChanged(old_state, new_state))
                    .unwrap_or_default();
            }
            PlayerComponentCommand::VolumeChanged(vol) => {
                sender
                    .output(PlayerComponentOutput::VolumeChanged(vol))
                    .unwrap_or_default();
            }
            PlayerComponentCommand::AudioVideoOffsetChanged(offset) => {
                sender
                    .output(PlayerComponentOutput::AudioVideoOffsetChanged(offset))
                    .unwrap_or_default();
            }
            PlayerComponentCommand::SubtitleVideoOffsetChanged(offset) => {
                sender
                    .output(PlayerComponentOutput::SubtitleVideoOffsetChanged(offset))
                    .unwrap_or_default();
            }
            PlayerComponentCommand::Warning(error) => {
                sender.output(PlayerComponentOutput::Warning(error)).unwrap_or_default();
            }
            PlayerComponentCommand::Error(error) => {
                sender.output(PlayerComponentOutput::Error(error)).unwrap_or_default();
            }
        }
    }
}

impl PlayerComponentModel {
    fn set_zoom(
        &mut self,
        new_scale: Option<f64>,
        video_scrolled_window: &mut gtk::ScrolledWindow,
        video_picture: &mut gtk::Picture,
    ) {
        let mut view_data = self.view_data.lock().unwrap();

        let old_zoom = view_data.video_view.zoom_factor;
        let new_scale = new_scale.unwrap_or(1.0).clamp(1.0, 10.0);
        trace!("New zoom: {}", new_scale);

        view_data
            .video_view
            .update(Some(new_scale), video_scrolled_window, video_picture);

        if view_data.video_view.zoom_factor == 1.0 {
            video_picture.set_width_request(0);
            video_picture.set_height_request(0);
            view_data.video_view.set_cursor(None);
        } else {
            trace!("paintable rectangle: {:?}", view_data.video_view.scaled_paintable_rect);
            trace!("view rectangle: {:?}", view_data.video_view.view_rect);
            trace!(
                "scrolled window: ha_upper={} va_upper={}",
                video_scrolled_window.hadjustment().upper(),
                video_scrolled_window.vadjustment().upper()
            );

            let fitted_paintable_rect = view_data.video_view.fitted_paintable_rect.clone().unwrap();

            trace!("Zoom video to {fitted_paintable_rect:?}");

            view_data.video_view.set_cursor(Some("grab"));
            video_picture.set_width_request(fitted_paintable_rect.w);
            video_picture.set_height_request(fitted_paintable_rect.h);

            // Adjust scrolled window viewport to the mouse position:
            if let Some((x, y)) = self.mouse_position {
                let ha = video_scrolled_window.hadjustment();
                let va = video_scrolled_window.vadjustment();

                // Adjust the scrollbar range to the new zoom level.
                // It is very important to have one step for rescaling and updating
                // the viewport. If we wait for the upper value of each scrollbar to be
                // updated, the video image flickers on each zoom.
                ha.set_upper(fitted_paintable_rect.w as f64);
                va.set_upper(fitted_paintable_rect.h as f64);

                // Translate the relative pointer position to the actual video image coordinates:
                let view_point = video_scrolled_window
                    .compute_point(video_picture, &gtk::graphene::Point::new(x as f32, y as f32))
                    .expect("Cannot translate x/y");
                let dst_x = view_point.x() as f64;
                let dst_y = view_point.y() as f64;

                // Make the pointer position unscaled:
                let dst_x = dst_x / old_zoom;
                let dst_y = dst_y / old_zoom;

                // Calculate the new x and y values of the scrolled video view:
                let new_content_x = ha.value() - (dst_x * old_zoom - dst_x * view_data.video_view.zoom_factor);
                let new_content_y = va.value() - (dst_y * old_zoom - dst_y * view_data.video_view.zoom_factor);

                trace!("└── move viewport to x={new_content_x} y={new_content_y}");

                // Update scrolled window position:
                ha.set_value(new_content_x);
                va.set_value(new_content_y);
            }
        }
    }

    fn new_gesture_drag(&self, sender: ComponentSender<Self>) -> gtk::GestureDrag {
        let drag = gtk::GestureDrag::builder().button(gtk::gdk::BUTTON_PRIMARY).build();

        drag.connect_drag_begin(clone!(
            #[strong]
            sender,
            move |_, x, y| {
                trace!("Scrolling: Drag begin x={x} y={y}");
                sender.input(PlayerComponentInput::PrivateMessage(PrivateMsg::DragBegin(x, y)));
            }
        ));
        drag.connect_drag_update(clone!(
            #[strong]
            sender,
            move |_, x, y| {
                sender.input(PlayerComponentInput::PrivateMessage(PrivateMsg::DragUpdate(x, y)));
            }
        ));

        drag.connect_drag_end(clone!(
            #[strong]
            sender,
            move |_, x, y| {
                sender.input(PlayerComponentInput::PrivateMessage(PrivateMsg::DragEnd(x, y)));
            }
        ));

        drag
    }

    fn new_motion_tracker(&self, sender: ComponentSender<Self>) -> gtk::EventControllerMotion {
        let tracker = gtk::EventControllerMotion::builder().build();

        tracker.connect_motion(clone!(
            #[strong]
            sender,
            move |_, x, y| {
                sender.input(PlayerComponentInput::PrivateMessage(PrivateMsg::MotionDetected(x, y)));
            }
        ));

        tracker
    }

    fn new_wheel_zoom(&self, sender: ComponentSender<Self>) -> gtk::EventControllerScroll {
        let zoom = gtk::EventControllerScroll::builder()
            .flags(gtk::EventControllerScrollFlags::VERTICAL)
            .build();

        zoom.connect_scroll(clone!(
            #[strong]
            sender,
            move |_, _, y| {
                let scale = (y / 10.0/* smooth scaling */) * SCALE_MULTIPLIER;
                // Invert scale to get a natural zoom experience:
                let scale = -scale;
                sender.input(PlayerComponentInput::SetZoomRelative(scale));
                gtk::glib::Propagation::Stop
            }
        ));

        zoom
    }
}
