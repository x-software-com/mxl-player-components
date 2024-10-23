use mxl_relm4_components::relm4::{self, adw::prelude::*, gtk::glib::clone, prelude::*};

use super::{
    messages::{CodecRankingComponentInput, CodecRankingComponentOutput},
    model::CodecRankingComponentModel,
};

#[relm4::component(pub)]
impl Component for CodecRankingComponentModel {
    type Init = super::CodecRankingComponentInit;
    type Input = CodecRankingComponentInput;
    type Output = CodecRankingComponentOutput;
    type CommandOutput = ();

    view! {
        #[name(pref_group)]
        adw::PreferencesGroup {
        }
    }

    // Initialize the component.
    fn init(init: Self::Init, root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let model = CodecRankingComponentModel {};

        let widgets = view_output!();

        widgets.pref_group.set_title(&init.title);

        for ci in init.codec_info_list {
            let switch = gtk::Switch::builder()
                .valign(gtk::Align::Center)
                .state(ci.enabled)
                .active(ci.enabled)
                .build();
            switch.connect_state_notify(clone!(
                #[strong]
                sender,
                #[strong(rename_to = codec_info)]
                ci.clone(),
                move |switch| {
                    if switch.state() {
                        sender.input(Self::Input::SetRank(codec_info.name.clone(), codec_info.default_rank));
                    } else {
                        sender.input(Self::Input::SetRank(codec_info.name.clone(), gst::Rank::NONE));
                    }
                }
            ));
            let row = adw::ActionRow::builder()
                .title(ci.name.clone())
                .subtitle(ci.long_name.clone())
                .activatable(true)
                .activatable_widget(&switch)
                .build();
            row.add_suffix(&switch);
            widgets.pref_group.add(&row);
        }

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, _: &Self::Root) {
        match msg {
            CodecRankingComponentInput::SetRank(name, rank) => {
                sender
                    .output_sender()
                    .emit(CodecRankingComponentOutput::SetRank(name, rank));
            }
        }
    }
}
