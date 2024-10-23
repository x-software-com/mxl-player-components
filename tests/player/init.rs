use anyhow::Result;
use log::*;
use mxl_player_components::*;
use mxl_relm4_components::relm4::{self, gtk::glib};
use tempfile::tempdir;

use super::about::APP_ID;

pub fn init() -> Result<()> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Trace)
        .is_test(true)
        .try_init()?;

    let tmp_dir = tempdir()?;

    let proc_dir = tmp_dir.path().join("proc");
    let cache_dir = tmp_dir.path().join("cache");

    glib_helpers::init_logging();
    glib::set_program_name(Some(APP_ID));
    mxl_relm4_components::init()?;

    mxl_player_components::init(&proc_dir, &cache_dir)?;

    relm4::RELM_THREADS.set(4).map_or_else(
        |size| {
            error!("Cannot set REALM_THREADS to '{}'", size);
        },
        |_| (),
    );
    Ok(())
}
