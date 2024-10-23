use gst::prelude::*;

pub struct CodecRankingComponentInit {
    pub title: String,
    pub codec_info_list: CodecInfoList,
}

pub struct CodecRankingComponentModel {}

#[derive(Clone)]
pub struct CodecInfo {
    pub name: String,
    pub long_name: String,
    pub default_rank: gst::Rank,
    pub enabled: bool,
}

pub type CodecInfoList = Vec<CodecInfo>;

pub struct CodecInfoListBuilder {
    codec_infos: Vec<CodecInfo>,
}

impl CodecInfoListBuilder {
    pub fn new(factories: gst::glib::List<gst::ElementFactory>) -> Self {
        let a = factories
            .iter()
            .map(|i| CodecInfo {
                name: i.name().into(),
                long_name: i.longname().into(),
                default_rank: i.rank(),
                enabled: true,
            })
            .collect();

        Self { codec_infos: a }
    }

    pub fn ignore(&mut self, name: String) -> &mut Self {
        if let Some(index) = self.codec_infos.iter().position(|i| i.name == name) {
            self.codec_infos.remove(index);
        }
        self
    }

    pub fn build(&self) -> CodecInfoList {
        self.codec_infos.clone()
    }
}
