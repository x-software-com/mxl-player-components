#[derive(Debug)]
pub enum CodecRankingComponentInput {
    SetRank(String, gst::Rank),
}

#[derive(Debug)]
pub enum CodecRankingComponentOutput {
    SetRank(String, gst::Rank),
}
