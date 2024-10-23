use anyhow::{Context, Error, Result};
use chrono::DateTime;
use gst::TagList;
use gst_pbutils::{prelude::*, DiscovererInfo, DiscovererResult};
use log::*;
use mxl_relm4_components::relm4::{
    self,
    factory::FactoryView,
    gtk::{glib, pango, prelude::*},
    prelude::*,
};
use notify_debouncer_mini::{new_debouncer, notify::*, DebounceEventResult, Debouncer};
use relm4_icons::icon_names;
use std::path::{Path, PathBuf};

use glib::clone;

use crate::localization::helper::fl;

#[derive(Debug, Clone, PartialEq)]
pub enum DropState {
    None,
    Above,
    Below,
}

#[derive(Debug)]
pub struct PlaylistEntryInit {
    pub uri: String,
    pub short_uri: Option<String>,
    pub error: Option<Error>,
}

pub struct PlaylistEntryModel {
    pub index: DynamicIndex,
    pub updating: bool,
    pub position: usize,
    active: bool,
    pub short_uri: String,
    pub uri: String,
    pub duration_text: String,
    pub info_text: String,
    pub info_tooltip: Option<String>,
    pub date_time: Option<DateTime<chrono::Local>>,
    pub error: Option<Error>,
    pub duration: Option<f64>,
    pub media_info: Option<DiscovererInfo>,
    pub notify_debouncer: Option<Debouncer<RecommendedWatcher>>,
}

#[derive(Debug, Clone)]
pub enum PlaylistEntryInput {
    Remove(DynamicIndex),
    Activate,
    Deactivate,
    FetchMetadata,
    SetDropState(DropState),
    EnterEvent,
    LeaveEvent,
}

#[derive(Debug)]
pub enum PlaylistEntryOutput {
    RemoveItem(DynamicIndex),
    Updated(DynamicIndex),
    Move(DynamicIndex, usize),
    AddBefore(DynamicIndex, Vec<PathBuf>),
    AddAfter(DynamicIndex, Vec<PathBuf>),
}

#[derive(Debug)]
pub enum PlaylistEntryCommandOutput {
    UpdateMetadata(Result<DiscovererInfo>),
}

const NOTIFY_TIMEOUT_SECS: u64 = 2;
const SPACING: i32 = 12;
const MARGIN: i32 = 4;

#[relm4::factory(pub)]
impl FactoryComponent for PlaylistEntryModel {
    type ParentWidget = gtk::ListBox;
    type Input = PlaylistEntryInput;
    type Output = PlaylistEntryOutput;
    type Init = PlaylistEntryInit;
    type CommandOutput = PlaylistEntryCommandOutput;

    view! {
        #[root]
        gtk::ListBoxRow {
            gtk::Box {
                set_hexpand: true,
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 0,

                #[name(above)]
                gtk::Separator {
                    set_hexpand: true,
                    set_orientation: gtk::Orientation::Horizontal,
                    set_height_request: 8,
                    set_css_classes: &["spacer"],
                },

                #[name(entry_box)]
                gtk::Box {
                    set_valign: gtk::Align::Center,
                    set_hexpand: true,
                    set_spacing: SPACING,
                    set_margin_all: MARGIN,
                    add_css_class: "activatable",

                    #[name(spinner)]
                    gtk::Spinner {
                        set_valign: gtk::Align::Center,
                        #[watch]
                        set_spinning: self.updating,
                        #[watch]
                        set_visible: self.updating,
                    },

                    #[name(icon)]
                    gtk::Image {
                        set_valign: gtk::Align::Center,
                        #[watch]
                        set_visible: !self.updating,
                        #[watch]
                        set_icon_name: if self.active {
                                Some(icon_names::PLAY_LARGE)
                            } else if self.error.is_some() {
                                Some(icon_names::WARNING)
                            } else {
                                None
                            },
                    },

                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,

                        gtk::Box {
                            set_orientation: gtk::Orientation::Horizontal,
                            set_hexpand: true,
                            set_spacing: SPACING * 2,

                            #[name(file_name)]
                            gtk::Label {
                                set_hexpand: true,
                                set_halign: gtk::Align::Start,
                                set_ellipsize: pango::EllipsizeMode::Middle,

                                #[watch]
                                set_css_classes: if self.active {
                                    &["accent"]
                                } else {
                                    &[]
                                },

                                #[watch]
                                set_markup: &format!("<b>{}</b>", self.short_uri),
                                #[watch]
                                set_tooltip_text: Some(&self.uri),
                            },

                            #[name(duration)]
                            gtk::Label {
                                #[watch]
                                set_css_classes: if self.active {
                                    &["accent"]
                                } else {
                                    &[]
                                },

                                #[watch]
                                set_markup: &self.duration_text,
                            },
                        },

                        #[name(info)]
                        gtk::Label {
                            set_hexpand: true,
                            set_halign: gtk::Align::Start,
                            set_ellipsize: pango::EllipsizeMode::End,

                            #[watch]
                            set_css_classes: if self.active {
                                &["accent"]
                            } else {
                                &[]
                            },

                            #[watch]
                            set_markup: &self.info_text,
                            #[watch]
                            set_tooltip_text: Some(self.info_tooltip.as_ref().unwrap_or(&"".to_owned())),
                        },
                    },

                    #[name(remove_button_revealer)]
                    gtk::Revealer {
                        set_transition_type: gtk::RevealerTransitionType::SlideLeft,

                        gtk::Button {
                            set_icon_name: icon_names::SMALL_X,
                            set_tooltip_text: Some(&fl!("remove-file", "desc")),
                            add_css_class: "destructive-action",
                            set_valign: gtk::Align::Center,
                            connect_clicked[sender, index] => move |_| {
                                    sender.input(PlaylistEntryInput::Remove(index.clone()))
                            }
                        }
                    }

                },

                #[name(below)]
                gtk::Separator {
                    set_hexpand: true,
                    set_orientation: gtk::Orientation::Horizontal,
                    set_height_request: 8,
                    set_css_classes: &["spacer"],
                },
            },
        }
    }

    fn init_model(init: Self::Init, index: &DynamicIndex, sender: FactorySender<Self>) -> Self {
        let notify_debouncer = match Self::init_file_watcher(&init.uri, sender)
            .with_context(|| format!("Cannot add watcher for file with uri '{}'", init.uri))
        {
            Ok(debouncer) => Some(debouncer),
            Err(error) => {
                error!("{:?}", error);
                None
            }
        };

        Self {
            index: index.clone(),
            updating: false,
            position: 0,
            active: false,
            short_uri: init.short_uri.unwrap_or(init.uri.clone()),
            uri: init.uri,
            duration_text: "".to_owned(),
            info_text: "".to_owned(),
            info_tooltip: None,
            date_time: None,
            error: init.error,
            duration: None,
            media_info: None,
            notify_debouncer,
        }
    }

    fn init_widgets(
        &mut self,
        index: &DynamicIndex,
        root: Self::Root,
        _returned_widget: &<Self::ParentWidget as FactoryView>::ReturnedWidget,
        sender: FactorySender<Self>,
    ) -> Self::Widgets {
        let widgets = view_output!();
        sender.input(PlaylistEntryInput::FetchMetadata);

        // Add controller to get enter and and leave events:
        let event_manager = gtk::EventControllerMotion::builder().build();
        event_manager.connect_enter(clone!(
            #[strong]
            sender,
            move |_, _, _| {
                sender.input(PlaylistEntryInput::EnterEvent);
            }
        ));
        event_manager.connect_leave(clone!(
            #[strong]
            sender,
            move |_| {
                sender.input(PlaylistEntryInput::LeaveEvent);
            }
        ));
        root.add_controller(event_manager);

        // Add controller to get key clicks to remove the current entry:
        let event_manager = gtk::EventControllerKey::builder().build();
        event_manager.connect_key_pressed(clone!(
            #[strong]
            sender,
            #[strong]
            index,
            move |_widget, key, _keycode, _modifier| {
                if key == gtk::gdk::Key::Delete || key == gtk::gdk::Key::BackSpace {
                    sender.input(PlaylistEntryInput::Remove(index.clone()));
                    return gtk::glib::Propagation::Stop;
                }
                gtk::glib::Propagation::Proceed
            }
        ));
        root.add_controller(event_manager);

        // Add drang & drop support:
        let drag_source = gtk::DragSource::builder().actions(gtk::gdk::DragAction::MOVE).build();
        drag_source.connect_prepare(clone!(
            #[strong(rename_to = own_index)]
            index,
            move |_drag_source, _x, _y| {
                Some(gtk::gdk::ContentProvider::for_value(
                    &glib::BoxedAnyObject::new(own_index.clone()).to_value(),
                ))
            }
        ));
        drag_source.connect_begin(clone!(
            #[weak]
            root,
            move |drag_source, _event| {
                let paintable = gtk::WidgetPaintable::new(Some(&root));
                drag_source.set_icon(Some(&paintable), 0, 0);
            }
        ));
        root.add_controller(drag_source);

        let drop_target = gtk::DropTarget::builder().actions(gtk::gdk::DragAction::MOVE).build();
        drop_target.connect_leave(clone!(
            #[strong]
            sender,
            move |_drop_target| {
                sender.input(PlaylistEntryInput::SetDropState(DropState::None));
            }
        ));
        drop_target.set_types(&[glib::BoxedAnyObject::static_type()]);
        drop_target.connect_drop(clone!(
            #[strong]
            sender,
            #[strong(rename_to = own_index)]
            index,
            #[weak(rename_to = self_widget)]
            root,
            #[upgrade_or]
            false,
            move |_drop_target, value, _x, y| {
                sender.input(PlaylistEntryInput::SetDropState(DropState::None));
                if let Ok(other_index) = value.get::<glib::BoxedAnyObject>() {
                    if let Ok(other_index) = other_index.try_borrow::<DynamicIndex>() {
                        if own_index.current_index() != other_index.current_index() {
                            let to = if y > self_widget.height() as f64 / 2.0 {
                                // move after own_index
                                if own_index.current_index() > other_index.current_index() {
                                    own_index.current_index()
                                } else {
                                    own_index.current_index() + 1
                                }
                            } else {
                                // move before own_index
                                if own_index.current_index() > other_index.current_index() {
                                    own_index.current_index() - 1
                                } else {
                                    own_index.current_index()
                                }
                            };

                            sender
                                .output(PlaylistEntryOutput::Move(other_index.clone(), to))
                                .unwrap_or_default();
                            return true;
                        }
                    }
                }
                false
            }
        ));
        drop_target.connect_motion(clone!(
            #[strong]
            sender,
            #[weak(rename_to = self_widget)]
            root,
            #[upgrade_or]
            gtk::gdk::DragAction::MOVE,
            move |_drop_target, _x, y| {
                if y > self_widget.height() as f64 / 2.0 {
                    sender.input(PlaylistEntryInput::SetDropState(DropState::Below));
                } else {
                    sender.input(PlaylistEntryInput::SetDropState(DropState::Above));
                }
                gtk::gdk::DragAction::MOVE
            }
        ));
        drop_target.connect_accept(|_drop_target, _drop| true);
        root.add_controller(drop_target);

        let formats = gtk::gdk::ContentFormatsBuilder::new()
            .add_type(gtk::gdk::FileList::static_type())
            .add_type(gtk::gio::File::static_type())
            .build();
        let drop_target = gtk::DropTarget::builder()
            .actions(gtk::gdk::DragAction::COPY)
            .formats(&formats)
            .build();
        drop_target.set_types(&[gtk::gdk::FileList::static_type(), gtk::gio::File::static_type()]);
        drop_target.connect_drop(clone!(
            #[strong]
            index,
            #[weak(rename_to = self_widget)]
            root,
            #[upgrade_or]
            false,
            move |_, value, _x, y| {
                let files = if let Ok(files) = value.get::<gtk::gdk::FileList>() {
                    let files: Vec<_> = files.files().iter().filter_map(|file| file.path()).collect();
                    Some(files)
                } else if let Ok(file) = value.get::<gtk::gio::File>() {
                    file.path().map(|file| vec![file])
                } else {
                    None
                };
                if let Some(files) = files {
                    sender.input(PlaylistEntryInput::SetDropState(DropState::None));
                    if y > self_widget.height() as f64 / 2.0 {
                        // add after own index
                        sender
                            .output(PlaylistEntryOutput::AddAfter(index.clone(), files))
                            .unwrap_or_default();
                    } else {
                        // add before own index
                        sender
                            .output(PlaylistEntryOutput::AddBefore(index.clone(), files))
                            .unwrap_or_default();
                    }
                    return true;
                }
                false
            }
        ));
        root.add_controller(drop_target);

        widgets
    }

    fn update_with_view(&mut self, widgets: &mut Self::Widgets, message: Self::Input, sender: FactorySender<Self>) {
        match message {
            PlaylistEntryInput::Remove(index) => sender
                .output(PlaylistEntryOutput::RemoveItem(index))
                .unwrap_or_default(),
            PlaylistEntryInput::Activate => {
                self.active = true;
            }
            PlaylistEntryInput::Deactivate => {
                self.active = false;
            }
            PlaylistEntryInput::FetchMetadata => {
                self.updating = true;
                let uri = self.uri.clone();
                sender.oneshot_command(async move {
                    let result = get_media_info(&uri);
                    PlaylistEntryCommandOutput::UpdateMetadata(result)
                });
            }
            PlaylistEntryInput::SetDropState(state) => match state {
                DropState::None => {
                    widgets.above.set_css_classes(&["spacer"]);
                    widgets.below.set_css_classes(&["spacer"]);
                }
                DropState::Above => {
                    widgets.above.set_css_classes(&[]);
                    widgets.below.set_css_classes(&["spacer"]);
                }
                DropState::Below => {
                    widgets.above.set_css_classes(&["spacer"]);
                    widgets.below.set_css_classes(&[]);
                }
            },
            PlaylistEntryInput::EnterEvent => {
                widgets.remove_button_revealer.set_reveal_child(true);
            }
            PlaylistEntryInput::LeaveEvent => {
                widgets.remove_button_revealer.set_reveal_child(false);
            }
        }
        self.update_view(widgets, sender)
    }

    fn update_cmd(&mut self, message: Self::CommandOutput, sender: FactorySender<Self>) {
        match message {
            PlaylistEntryCommandOutput::UpdateMetadata(result) => {
                self.updating = false;
                self.duration = None;
                "".clone_into(&mut self.duration_text);
                self.error = None;
                self.info_tooltip = None;
                match result {
                    Err(error) => self.error = Some(error),
                    Ok(info) => {
                        trace_media_info(&info);
                        self.uri = info.uri().to_string();
                        match info.result() {
                            DiscovererResult::Ok => {
                                if let Some(duration) = info.duration() {
                                    self.duration = Some(duration.mseconds() as f64 / 1000_f64);
                                    self.duration_text =
                                        format!("<span font_desc=\"monospace\">{:.0}</span>", duration);
                                }
                                if let Some(info) = info.stream_info() {
                                    if let Some(info) = info.downcast_ref::<gst_pbutils::DiscovererContainerInfo>() {
                                        if let Some(tags) = info.tags() {
                                            if let Some(date_time) = tags.get::<gst::tags::DateTime>() {
                                                match date_time.get().to_iso8601_string() {
                                                    Ok(iso_string) => {
                                                        match iso_string.parse::<DateTime<chrono::Local>>() {
                                                            Ok(chrono_time) => {
                                                                self.info_text = chrono_time.to_rfc2822();
                                                                self.date_time = Some(chrono_time);
                                                            }
                                                            Err(_) => self.info_text = format!("{}", date_time.get()),
                                                        }
                                                    }
                                                    Err(_) => self.info_text = format!("{}", date_time.get()),
                                                }
                                            } else {
                                                "".clone_into(&mut self.info_text);
                                            }
                                        }
                                    }
                                }
                            }
                            DiscovererResult::MissingPlugins => {
                                let details: Vec<_> = info
                                    .missing_elements_installer_details()
                                    .iter()
                                    .map(|x| x.to_string())
                                    .collect();
                                self.error = Some(anyhow::anyhow!("{}", details.join(", ")));
                            }
                            DiscovererResult::UriInvalid => {
                                self.error = Some(anyhow::anyhow!(fl!("invalid-uri", uri = self.uri.clone())));
                            }
                            DiscovererResult::Timeout => {
                                self.error = Some(anyhow::anyhow!(fl!("file-discovery-timeout")));
                            }
                            DiscovererResult::Busy => unreachable!(),
                            DiscovererResult::Error => unreachable!(),
                            _ => (),
                        }
                        self.media_info = Some(info);
                    }
                }
                if let Some(error) = &self.error {
                    self.info_text = format!("{error:?}");
                    self.info_tooltip = Some(self.info_text.clone())
                }
                sender
                    .output(PlaylistEntryOutput::Updated(self.index.clone()))
                    .unwrap_or_default();
            }
        }
    }
}

impl PlaylistEntryModel {
    fn init_file_watcher(uri_str: &str, sender: FactorySender<Self>) -> Result<Debouncer<RecommendedWatcher>> {
        let uri = relm4::gtk::glib::Uri::parse(uri_str, relm4::gtk::glib::UriFlags::PARSE_RELAXED)?;
        let file_path = uri.path().to_string();
        let mut debouncer: Debouncer<RecommendedWatcher> = new_debouncer(
            std::time::Duration::from_secs(NOTIFY_TIMEOUT_SECS),
            move |res: DebounceEventResult| match res {
                Ok(events) => events.iter().for_each(|e| {
                    debug!("File {:?} changed, updating metadata", e.path);
                    sender.input(PlaylistEntryInput::FetchMetadata);
                }),
                Err(error) => error!("Error {:?}", error),
            },
        )?;
        debouncer
            .watcher()
            .watch(Path::new(&file_path), RecursiveMode::NonRecursive)?;

        Ok(debouncer)
    }
}

fn get_media_info(uri: &str) -> Result<DiscovererInfo> {
    let timeout: gst::ClockTime = gst::ClockTime::from_seconds(10);
    let discoverer = gst_pbutils::Discoverer::new(timeout)?;
    let info = discoverer.discover_uri(uri)?;

    Ok(info)
}

fn trace_media_info(info: &DiscovererInfo) {
    let mut tree = termtree::Tree::new(format!("URI: {}", info.uri()));
    match info.result() {
        DiscovererResult::Ok => {
            tree.push(termtree::Tree::new(format!("is live: {}", info.is_live())));
            tree.push(termtree::Tree::new(format!("is seekable: {}", info.is_seekable())));
            if let Some(duration) = info.duration() {
                tree.push(termtree::Tree::new(format!("Duration: {:.0}", duration)));
            }
            if let Some(info) = info.toc() {
                let mut sub_tree = termtree::Tree::new("TOC:".to_owned());
                for entry in info.entries() {
                    sub_tree.push(trace_media_toc_entry(&entry));
                }
                tree.push(sub_tree);
            }
            if let Some(info) = info.stream_info() {
                if let Some(info) = info.downcast_ref::<gst_pbutils::DiscovererContainerInfo>() {
                    let mut sub_tree = termtree::Tree::new(format!("Stream #{}", info.stream_number()));
                    sub_tree.push(termtree::Tree::new(format!("type: {}", info.stream_type_nick())));
                    if let Some(tags) = info.tags() {
                        sub_tree.push(trace_media_info_tags(&tags));
                    }
                    tree.push(sub_tree);
                }
            }
            for stream in info.container_streams() {
                let mut sub_tree = termtree::Tree::new(format!("Stream #{}", stream.stream_number()));
                sub_tree.push(termtree::Tree::new(format!("type: {}", stream.stream_type_nick())));
                if let Some(tags) = stream.tags() {
                    sub_tree.push(trace_media_info_tags(&tags));
                }
                tree.push(sub_tree);
            }
            for stream in info.stream_list() {
                let mut sub_tree = termtree::Tree::new(format!("Stream #{}", stream.stream_number()));
                sub_tree.push(termtree::Tree::new(format!("type: {}", stream.stream_type_nick())));
                sub_tree.push(termtree::Tree::new(format!("id: {:?}", stream.stream_id())));
                if let Some(tags) = stream.tags() {
                    sub_tree.push(trace_media_info_tags(&tags));
                }
                tree.push(sub_tree);
            }
        }
        DiscovererResult::MissingPlugins => {
            for missing_info in info.missing_elements_installer_details() {
                tree.push(termtree::Tree::new(format!("{missing_info}")));
            }
        }
        DiscovererResult::UriInvalid => {
            tree.push(termtree::Tree::new(format!("Invalid uri {}", info.uri())));
        }
        DiscovererResult::Timeout => {
            tree.push(termtree::Tree::new("File info discovery timed out".to_owned()));
        }
        DiscovererResult::Busy => {
            tree.push(termtree::Tree::new("Discoverer is busy".to_owned()));
        }
        DiscovererResult::Error => {
            tree.push(termtree::Tree::new("Discoverer error".to_owned()));
        }
        _ => (),
    }
    for line in tree.to_string().lines() {
        trace!("{line}")
    }
}

fn trace_media_toc_entry(entry: &gst::TocEntry) -> termtree::Tree<String> {
    let mut tree = termtree::Tree::new("Entry:".to_owned());

    tree.push(termtree::Tree::new(format!("type: {}", entry.entry_type().nick())));
    if let Some(tags) = entry.tags() {
        tree.push(trace_media_info_tags(&tags));
    }
    for sub_entry in entry.sub_entries() {
        tree.push(trace_media_toc_entry(&sub_entry));
    }

    tree
}

fn trace_media_info_tag(name: &str, value: &gst::glib::value::SendValue, tree: &mut termtree::Tree<String>) {
    let get_tag_value = |value: &gst::glib::value::SendValue| -> Option<String> {
        if let Ok(s) = value.get::<&str>() {
            Some(s.to_string())
        } else {
            None
        }
    };

    if let Some(value) = get_tag_value(value) {
        if name == gst::tags::ExtendedComment::TAG_NAME {
            if let Ok(ext_comment) = gst_tag::tag_parse_extended_comment(&value, true) {
                tree.push(termtree::Tree::new(format!(
                    "{}: {}",
                    ext_comment.key.expect("Expect key in extended comment"),
                    ext_comment.value
                )));
            };
        } else {
            tree.push(termtree::Tree::new(format!("{}: {}", name, value)));
        }
    }
}

fn trace_media_info_tags(tags: &TagList) -> termtree::Tree<String> {
    let mut tree = termtree::Tree::new("Tags:".to_owned());

    for (name, values) in tags.iter_generic() {
        if name == gst::tags::DateTime::TAG_NAME {
            if let Some(date_time) = tags.get::<gst::tags::DateTime>() {
                if let Ok(iso_date_time) = date_time.get().to_iso8601_string() {
                    if let Ok(date_time) = iso_date_time.parse::<DateTime<chrono::Local>>() {
                        tree.push(termtree::Tree::new(format!("{}: {}", name, date_time.to_rfc2822())));
                    }
                }
            }
        } else {
            for value in values {
                trace_media_info_tag(name, value, &mut tree);
            }
        }
    }
    tree
}
