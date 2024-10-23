use mxl_relm4_components::relm4::gtk::glib;

fn from_glib_level(level: &glib::LogLevel) -> (&'static str, log::Level) {
    match &level {
        glib::LogLevel::Critical => ("critical", log::Level::Error),
        glib::LogLevel::Error => ("error", log::Level::Error),
        glib::LogLevel::Warning => ("warning", log::Level::Warn),
        glib::LogLevel::Info => ("info", log::Level::Info),
        glib::LogLevel::Debug => ("debug", log::Level::Debug),
        glib::LogLevel::Message => ("message", log::Level::Trace),
    }
}

pub fn init_logging() {
    macro_rules! get_module_path {
        ($target_buf:expr, $target:expr, $domain:expr) => {
            if let Some(domain) = $domain {
                $target_buf = format!("{}::{}", $target, domain);
                $target_buf.as_str()
            } else {
                $target
            }
        };
    }

    // Set logger for GLIB:
    glib::log_set_default_handler(|domain, level, message| {
        let target_buf;
        let module_path = get_module_path!(target_buf, "glib", domain);
        let (glib_level, level) = from_glib_level(&level);
        let target = format!("{}|{}", glib_level, module_path);

        log::logger().log(
            &log::RecordBuilder::new()
                .args(format_args!("{}", message))
                .level(level)
                .target(target.as_str())
                .module_path(Some(module_path))
                .build(),
        );
    });

    // Set logger for GLIB log writer:
    glib::log_set_writer_func(|level, log_fields| {
        macro_rules! get_field {
            ($target:expr) => {
                log_fields
                    .iter()
                    .find_map(|f| if f.key() == $target { f.value_str() } else { None })
            };
        }

        let message = get_field!("MESSAGE").unwrap_or("<empty message>");
        let file = get_field!("CODE_FILE");
        let line = get_field!("CODE_LINE").map_or(None, |s| s.parse::<u32>().ok());

        let mut misc_fields = vec![];
        macro_rules! get_misc_field {
            ($target:expr) => {
                if let Some(val) = get_field!($target) {
                    misc_fields.push(format!("{}={}", $target, val));
                }
            };
            ($target:expr, $target_synonym:expr) => {
                if let Some(val) = get_field!($target) {
                    misc_fields.push(format!("{}={}", $target_synonym, val));
                }
            };
        }
        get_misc_field!("PRIORITY");
        get_misc_field!("CODE_FUNC", "FUNC");

        let domain = get_field!("GLIB_DOMAIN");
        let target_buf;
        let module_path = get_module_path!(target_buf, "glib", domain);
        let (glib_level, level) = from_glib_level(&level);
        let target = format!("{}|{}|{}", glib_level, module_path, misc_fields.join("|"));

        log::logger().log(
            &log::RecordBuilder::new()
                .args(format_args!("{}", message))
                .level(level)
                .target(target.as_str())
                .module_path(Some(module_path))
                .file(file)
                .line(line)
                .build(),
        );

        glib::LogWriterOutput::Handled
    });
}
