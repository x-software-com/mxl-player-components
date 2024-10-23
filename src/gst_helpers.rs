use gst::glib;
use log::*;
use std::path::Path;

pub fn init(cache_dir: &Path) {
    // Set logger for GStreamer:
    gst::log::add_log_function(|category, level, file, _function, line, obj, message| {
        macro_rules! get_module_path {
            ($target:expr) => {
                if let Some(obj) = obj {
                    format!("{}::{}::{}", $target, category.name(), obj.to_string())
                } else {
                    format!("{}::{}", $target, category.name())
                }
            };
        }
        let module_path = get_module_path!("gst");

        let (gst_level, level) = match &level {
            gst::DebugLevel::None => ("none", log::Level::Trace),
            gst::DebugLevel::Error => ("error", log::Level::Error),
            gst::DebugLevel::Warning => ("warning", log::Level::Warn),
            gst::DebugLevel::Fixme => ("fixme", log::Level::Trace),
            gst::DebugLevel::Info => ("info", log::Level::Info),
            gst::DebugLevel::Debug => ("debug", log::Level::Debug),
            gst::DebugLevel::Log => ("log", log::Level::Trace),
            gst::DebugLevel::Trace => ("trace", log::Level::Trace),
            gst::DebugLevel::Memdump => ("memdump", log::Level::Trace),
            _ => ("unknown", log::Level::Trace),
        };

        let target = format!("{}|{}", gst_level, module_path);

        log::logger().log(
            &log::RecordBuilder::new()
                .args(format_args!(
                    "{}",
                    message.get().unwrap_or(glib::gstr!("Empty message").into())
                ))
                .level(level)
                .target(target.as_str())
                .module_path(Some(module_path.as_str()))
                .file(Some(file))
                .line(Some(line))
                .build(),
        );
    });

    // Remove default stderr logger:
    gst::log::remove_default_log_function();

    let registry = cache_dir.join("registry.bin");
    info!("Use registry file: {:?}", registry);
    std::env::set_var("GST_REGISTRY", registry);

    // Set debug level before gstreamer initialization
    if gst::log::get_default_threshold() < gst::DebugLevel::Warning {
        gst::log::set_default_threshold(gst::DebugLevel::Warning);
    }
}
