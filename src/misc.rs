use crate::gst_helpers;
use anyhow::Result;
use std::path::Path;

pub const ENV_NAME_GST_DEBUG_DUMP_DOT_DIR: &str = "GST_DEBUG_DUMP_DOT_DIR";

pub fn init(gst_debug_dump_dot_dir: &Path, cache_dir: &Path) -> Result<()> {
    crate::localization::init();
    mxl_relm4_components::init()?;

    std::env::set_var(ENV_NAME_GST_DEBUG_DUMP_DOT_DIR, gst_debug_dump_dot_dir);
    gst_helpers::init(cache_dir);
    gst::init()?;

    gstgtk4::plugin_register_static().expect("Failed to register the gstgtk4 plugin");

    Ok(())
}
