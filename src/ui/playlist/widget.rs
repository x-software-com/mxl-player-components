use log::*;
use mxl_relm4_components::relm4::{self, actions::*, adw::prelude::*, factory::FactoryVecDeque, gtk::glib, prelude::*};
use relm4_icons::icon_names;

use glib::clone;

use crate::localization::helper::fl;
use crate::ui::playlist::{
    messages::{
        PlaylistChange, PlaylistCommandOutput, PlaylistComponentInput, PlaylistComponentOutput, PlaylistState,
        RepeatMode, SortOrder,
    },
    model::{InsertMode, PlaylistComponentInit, PlaylistComponentModel},
};

use super::factory::{PlaylistEntryInput, PlaylistEntryOutput};

relm4::new_action_group!(SortActionGroup, "sort_action_group");
relm4::new_stateless_action!(SortByStartTime, SortActionGroup, "sort_by_start_time");
relm4::new_stateless_action!(SortByShortUri, SortActionGroup, "sort_by_short_uri");

#[relm4::component(pub)]
impl Component for PlaylistComponentModel {
    type Init = PlaylistComponentInit;
    type Input = PlaylistComponentInput;
    type Output = PlaylistComponentOutput;
    type CommandOutput = PlaylistCommandOutput;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_width_request: 650,
            set_css_classes: &["background"],

            adw::HeaderBar {
                set_css_classes: &["flat"],
                set_show_end_title_buttons: false,
                set_title_widget: Some(&gtk::Label::new(Some(&fl!("playlist")))),
                pack_start = &gtk::Button {
                    set_has_tooltip: true,
                    set_tooltip_text: Some(&fl!("add-file")),
                    set_icon_name: icon_names::PLUS,
                    set_css_classes: &["flat", "image-button"],
                    set_valign: gtk::Align::Center,
                    connect_clicked => PlaylistComponentInput::FileChooserRequest,
                },
                pack_end = &gtk::Button {
                    set_has_tooltip: true,
                    #[watch]
                    set_tooltip_text: Some(match model.repeat {
                            RepeatMode::Off => fl!("repeat", "none"),
                            RepeatMode::All => fl!("repeat", "all"),
                        }.as_ref()),
                    #[watch]
                    set_icon_name: match model.repeat {
                            RepeatMode::Off => icon_names::ARROW_REPEAT_ALL_OFF_FILLED,
                            RepeatMode::All => icon_names::ARROW_REPEAT_ALL_FILLED,
                        },
                    connect_clicked[sender] => move |_| {
                        sender.input(PlaylistComponentInput::ToggleRepeat);
                    }
                },
                 pack_end = &gtk::MenuButton {
                    set_label: &fl!("sort-by"),

                    set_menu_model: Some(&{
                        let menu_model = gtk::gio::Menu::new();
                        menu_model.append(
                            Some(&fl!("sort-by", "start-time")),
                            Some(&SortByStartTime::action_name()),
                        );
                        menu_model.append(
                            Some(&fl!("sort-by", "file-name")),
                            Some(&SortByShortUri::action_name()),
                        );
                        menu_model
                    }),
                }
            },

            #[name="drop_box"]
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_vexpand: true,

                gtk::ScrolledWindow {
                    #[watch]
                    set_visible: !model.show_placeholder,
                    set_hscrollbar_policy: gtk::PolicyType::Never,
                    set_vexpand: true,

                    #[local_ref]
                    file_list_box -> gtk::ListBox {
                        add_css_class: "boxed-list",
                        set_activate_on_single_click: false,
                        connect_row_activated[sender] => move |_, row| {
                            sender.input(PlaylistComponentInput::Activate(row.index() as usize))
                        }
                    }
                },
                adw::StatusPage {
                    #[watch]
                    set_visible: model.show_placeholder,
                    set_vexpand: true,
                    set_icon_name: Some(icon_names::VIDEO_CLIP_MULTIPLE_REGULAR),
                    set_title: &fl!("playlist-empty"),
                    set_description: Some(&fl!("playlist-empty", "desc")),
                }
            }
        }
    }

    // Initialize the component.
    fn init(init: Self::Init, root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let mut group = RelmActionGroup::<SortActionGroup>::new();
        group.add_action(RelmAction::<SortByStartTime>::new_stateless(clone!(
            #[strong]
            sender,
            move |_| {
                sender.input(PlaylistComponentInput::Sort(SortOrder::StartTime));
            }
        )));
        group.add_action({
            RelmAction::<SortByShortUri>::new_stateless(clone!(
                #[strong]
                sender,
                move |_| {
                    sender.input(PlaylistComponentInput::Sort(SortOrder::ShortUri));
                }
            ))
        });
        group.register_for_widget(&root);

        let uris =
            FactoryVecDeque::builder()
                .launch(gtk::ListBox::default())
                .forward(sender.input_sender(), |output| match output {
                    PlaylistEntryOutput::RemoveItem(index) => Self::Input::Remove(index),
                    PlaylistEntryOutput::Updated(index) => Self::Input::Updated(index),
                    PlaylistEntryOutput::Move(from, to) => Self::Input::Move(from, to),
                    PlaylistEntryOutput::AddBefore(index, files) => Self::Input::AddBefore(index, files),
                    PlaylistEntryOutput::AddAfter(index, files) => Self::Input::AddAfter(index, files),
                });

        let mut model = PlaylistComponentModel {
            uris,
            index: None,
            state: PlaylistState::Stopped,
            show_placeholder: init.uris.is_empty(),
            repeat: RepeatMode::Off,
        };

        model.add_uris(&sender, InsertMode::Back, &init.uris);

        let file_list_box = model.uris.widget();
        let widgets: PlaylistComponentModelWidgets = view_output!();
        widgets
            .drop_box
            .add_controller(PlaylistComponentModel::new_drop_target(sender.input_sender().clone()));

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, _: &Self::Root) {
        match msg {
            PlaylistComponentInput::Start => {
                debug!("Playlist start");
                if let Some(entry) = self.uris.guard().get(0) {
                    sender.input(PlaylistComponentInput::Switch(entry.index.clone()));
                }
            }
            PlaylistComponentInput::Stop => {
                self.state = PlaylistState::Stopping;
                sender
                    .output(PlaylistComponentOutput::StateChanged(PlaylistState::Stopping))
                    .unwrap_or_default();
            }
            PlaylistComponentInput::PlayerStopped => match self.state {
                PlaylistState::Stopping => {
                    self.uris.broadcast(PlaylistEntryInput::Deactivate);
                    self.index = None;
                    sender
                        .output(PlaylistComponentOutput::StateChanged(PlaylistState::Stopped))
                        .unwrap_or_default();
                }
                PlaylistState::Playing => (),
                PlaylistState::Stopped => (),
            },
            PlaylistComponentInput::PlayerPlaying => {
                self.state = PlaylistState::Playing;
            }
            PlaylistComponentInput::Previous => {
                self.previous(&sender);
            }
            PlaylistComponentInput::Next => {
                self.next(&sender);
            }
            PlaylistComponentInput::Activate(index) => {
                if let Some(entry) = self.uris.get(index) {
                    sender.input(PlaylistComponentInput::Switch(entry.index.clone()))
                }
            }
            PlaylistComponentInput::Switch(index) => {
                self.uris.broadcast(PlaylistEntryInput::Deactivate);
                self.uris.send(index.current_index(), PlaylistEntryInput::Activate);
                self.index = Some(index.clone());
                if let Some(entry) = self.uris.guard().get_mut(index.current_index()) {
                    sender
                        .output(PlaylistComponentOutput::SwitchUri(entry.uri.clone()))
                        .unwrap_or_default();
                }
            }
            PlaylistComponentInput::EndOfPlaylist(_index) => {
                self.uris.broadcast(PlaylistEntryInput::Deactivate);
                self.index = None;
                sender
                    .output(PlaylistComponentOutput::EndOfPlaylist)
                    .unwrap_or_default();
            }
            PlaylistComponentInput::Add(files) => {
                self.add_uris(&sender, InsertMode::Back, &files);
            }
            PlaylistComponentInput::AddBefore(index, files) => {
                self.add_uris(&sender, InsertMode::AtIndex(index), &files);
            }
            PlaylistComponentInput::AddAfter(index, files) => {
                let edit = self.uris.guard();
                if let Some(index) = index.current_index().checked_add(1) {
                    if let Some(index) = edit.get(index) {
                        let index = index.index.clone();
                        drop(edit);
                        self.add_uris(&sender, InsertMode::AtIndex(index), &files);
                    } else {
                        drop(edit);
                        self.add_uris(&sender, InsertMode::Back, &files);
                    }
                }
            }
            PlaylistComponentInput::Remove(index) => {
                debug!("Remove item {index:?}");
                if let Some(current_index) = self.index.clone() {
                    if index == current_index {
                        self.next(&sender);
                    }
                }
                self.uris.guard().remove(index.current_index());
                sender
                    .command_sender()
                    .emit(PlaylistCommandOutput::ShowPlaceholder(self.uris.guard().is_empty()));
                sender
                    .output_sender()
                    .emit(PlaylistComponentOutput::PlaylistChanged(PlaylistChange::Removed));
            }
            PlaylistComponentInput::Updated(index) => {
                sender
                    .output_sender()
                    .emit(PlaylistComponentOutput::PlaylistChanged(PlaylistChange::Updated));
                trace!("Updated item {}", index.current_index());
            }
            PlaylistComponentInput::Move(from, to) => {
                let mut edit = self.uris.guard();
                if let Some(to) = edit.get(to) {
                    let from = from.current_index();
                    let to = to.index.current_index();
                    trace!("Move playlist entry from index {} to {}", from, to);
                    edit.move_to(from, to);
                    sender
                        .output_sender()
                        .emit(PlaylistComponentOutput::PlaylistChanged(PlaylistChange::Reordered));
                }
            }
            PlaylistComponentInput::FetchMetadata => {
                self.uris.broadcast(PlaylistEntryInput::FetchMetadata);
            }
            PlaylistComponentInput::FileChooserRequest => {
                sender
                    .output(PlaylistComponentOutput::FileChooserRequest)
                    .unwrap_or_default();
            }
            PlaylistComponentInput::Sort(order) => {
                debug!("Sort playlist by {order:?}");
                self.sort_factory(&order);
                sender
                    .output_sender()
                    .emit(PlaylistComponentOutput::PlaylistChanged(PlaylistChange::Reordered));
            }
            PlaylistComponentInput::ToggleRepeat => {
                self.repeat = match self.repeat {
                    RepeatMode::Off => RepeatMode::All,
                    RepeatMode::All => RepeatMode::Off,
                };
                debug!("Change repeat to {:?}", self.repeat);
            }
        }
    }

    fn update_cmd(&mut self, msg: Self::CommandOutput, _sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            PlaylistCommandOutput::ShowPlaceholder(val) => {
                self.show_placeholder = val;
            }
        }
    }
}
