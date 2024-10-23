fn main() {
    //
    // Rerun cargo if one of the internationalization files change:
    //
    println!("cargo:rerun-if-changed=i18n.toml");
    println!("cargo:rerun-if-changed=i18n/en/mxl_player_components.ftl");
}
