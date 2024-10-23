use anyhow::Result;
use log::*;
use mxl_relm4_components::relm4::gtk::{
    gio::{prelude::FileExt, File},
    glib,
};
use std::path::Path;

pub fn uri_from_pathbuf(path: &Path) -> Result<String> {
    if let Some(path_string) = path.to_str() {
        let uri = if path.is_file() {
            let file_path = File::for_path(path);
            trace!(
                "file_path(:?) = {:?} file_path(uri) = {}",
                file_path,
                file_path.uri().as_str()
            );
            glib::Uri::parse(file_path.uri().as_str(), glib::UriFlags::PARSE_RELAXED)?
        } else {
            glib::Uri::parse(path_string, glib::UriFlags::PARSE_RELAXED)?
        };

        return Ok(uri.to_str().to_string());
    }
    Err(anyhow::anyhow!(
        "The path {} is not a valid URI",
        path.to_str().unwrap_or_default()
    ))
}
