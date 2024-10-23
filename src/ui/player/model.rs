use crate::player::{Player, PlayerBuilder};
use crate::ui::player::messages::PlaybackState;
use log::*;
use mxl_relm4_components::relm4::{gtk, gtk::prelude::*};
use std::{rc::Rc, sync::Mutex};

type DrawCallbackFn = dyn Fn(&gtk::cairo::Context, &mut VideoViewData);

pub struct PlayerComponentInit {
    pub seek_accurate: bool,
    pub show_seeking_overlay: bool,
    pub compositor: Option<gst::Element>,
    pub draw_callback: Box<DrawCallbackFn>,
    pub drag_gesture: Option<gtk::GestureDrag>,
    pub motion_tracker: Option<gtk::EventControllerMotion>,
}

#[derive(Debug, Default)]
pub struct VideoViewData {
    pub drawing_area: Option<gst_video::VideoRectangle>,
    pub view_rect: Option<gst_video::VideoRectangle>,
    pub video_dimensions: Option<gst_video::VideoRectangle>,
    pub scaled_paintable_rect: Option<gst_video::VideoRectangle>,
    pub fitted_paintable_rect: Option<gst_video::VideoRectangle>,
    pub zoom_factor: f64,
    pub(super) cursor_widgets: Vec<gtk::Widget>,
    cursor_name: Option<String>,
}

#[derive(Debug, Default)]
pub(super) struct ViewData {
    pub(super) video_view: VideoViewData,
}

pub(super) struct DrawCallbackData {
    pub(super) draw_callback: Box<DrawCallbackFn>,
}

impl DrawCallbackData {
    pub(super) fn new(callback: Box<DrawCallbackFn>) -> Self {
        Self {
            draw_callback: callback,
        }
    }
}

pub struct PlayerComponentModel {
    pub(super) player_builder: PlayerBuilder,
    pub(super) player: Option<Player>,
    pub(super) playback_state: PlaybackState,
    pub(super) show_seeking_overlay: bool,
    pub(super) seeking: bool,
    pub(super) show_drawing_overlay: bool,
    pub(super) view_data: Rc<Mutex<ViewData>>,
    pub(super) draw_callback: Rc<Mutex<DrawCallbackData>>,
    pub(super) drag_position: Option<(f64, f64)>,
    pub(super) mouse_position: Option<(f64, f64)>,
}

impl VideoViewData {
    pub(super) fn set_cursor_widgets(&mut self, video_view: Vec<gtk::Widget>) {
        self.cursor_widgets = video_view;
    }

    pub(super) fn set_cursor(&mut self, cursor_name: Option<&str>) {
        debug!("Video view set cursor: {cursor_name:?}");
        self.cursor_widgets.iter().for_each(|w| {
            w.set_cursor_from_name(cursor_name);
        });
        self.cursor_name = cursor_name.map(str::to_string);
    }

    pub fn set_custom_cursor_from_name(&mut self, cursor_name: Option<&str>) {
        if cursor_name.is_some() {
            debug!("Video view set custom cursor: {cursor_name:?}");
            self.cursor_widgets.iter().for_each(|w| {
                w.set_cursor_from_name(cursor_name);
            });
        } else {
            debug!("Video view reset current cursor: {:?}", self.cursor_name);
            self.cursor_widgets.iter().for_each(|w| {
                w.set_cursor_from_name(self.cursor_name.as_deref());
            });
        }
    }

    pub(super) fn update(
        &mut self,
        new_zoom_factor: Option<f64>,
        video_scrolled_window: &gtk::ScrolledWindow,
        video_picture: &gtk::Picture,
    ) {
        if let Some(new_zoom_factor) = new_zoom_factor {
            self.zoom_factor = new_zoom_factor;
        }

        // Unscaled size of the video picture from GStreamer:
        let p_width = video_picture.paintable().unwrap().intrinsic_width();
        let p_height = video_picture.paintable().unwrap().intrinsic_height();

        // let paintable_rect = gst_video::VideoRectangle::new(0, 0, p_width, p_height);
        let paintable_rect = gst_video::VideoRectangle::new(
            0,
            0,
            (p_width as f64 * self.zoom_factor) as i32,
            (p_height as f64 * self.zoom_factor) as i32,
        );
        self.scaled_paintable_rect = Some(paintable_rect.clone());

        let view_rect =
            gst_video::VideoRectangle::new(0, 0, video_scrolled_window.width(), video_scrolled_window.height());

        self.view_rect = Some(view_rect.clone());

        if self.zoom_factor == 1.0 {
            self.fitted_paintable_rect = Some(gst_video::center_video_rectangle(&paintable_rect, &view_rect, true));
        } else {
            trace!("paintable orig: p_width={p_width} p_height={p_height}");

            let new_view_w = (view_rect.w as f64 * self.zoom_factor) as i32;
            let new_view_h = (view_rect.h as f64 * self.zoom_factor) as i32;
            let new_view_rect = gst_video::VideoRectangle::new(0, 0, new_view_w, new_view_h);

            trace!("Zoomed view size: {new_view_rect:?}");

            let unscaled_fitted_paintable_rect = gst_video::center_video_rectangle(&paintable_rect, &view_rect, true);

            let new_paintable_w = (unscaled_fitted_paintable_rect.w as f64 * self.zoom_factor) as i32;
            let new_paintable_h = (unscaled_fitted_paintable_rect.h as f64 * self.zoom_factor) as i32;
            let fitted_paintable_rect = gst_video::VideoRectangle::new(0, 0, new_paintable_w, new_paintable_h);

            let fitted_paintable_rect = gst_video::center_video_rectangle(&fitted_paintable_rect, &new_view_rect, true);

            let fitted_paintable_rect = gst_video::VideoRectangle::new(
                ((view_rect.w - fitted_paintable_rect.w) / 2).clamp(0, i32::MAX),
                ((view_rect.h - fitted_paintable_rect.h) / 2).clamp(0, i32::MAX),
                fitted_paintable_rect.w,
                fitted_paintable_rect.h,
            );

            self.fitted_paintable_rect = Some(fitted_paintable_rect.clone());
        }
    }
}
