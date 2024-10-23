use const_format;

pub enum Accelerators {
    Quit,
    FileChooser,
    TogglePlaylistVisibility,
    TogglePlayPause,
    NextFrame,
    Previous,
    Next,
    IncreaseVolume,
    DecreaseVolume,
    IncreaseSpeed,
    DecreaseSpeed,
    ResetSpeed,
    FullScreen,
    DumpPipeline,
    Preferences,
    VideoOffsets,
}

#[macro_export]
#[cfg(target_os = "macos")]
macro_rules! action_accelerator_with_os_modifier {
    ($accelerator:expr) => {
        const_format::formatcp!("<Meta>{}", $accelerator)
    };
}

#[macro_export]
#[cfg(not(target_os = "macos"))]
macro_rules! action_accelerator_with_os_modifier {
    ($accelerator:expr) => {
        const_format::formatcp!("<Primary>{}", $accelerator)
    };
}

// The syntax of the accelerators and the modifiers is described here:
// https://gtk-rs.org/gtk4-rs/stable/latest/docs/gtk4/fn.accelerator_parse.html
//
// To browse all non modifier accelerator names see:
// https://gtk-rs.org/gtk4-rs/stable/latest/docs/src/gdk4/keys.rs.html#164
//
// To use a key name remove the 'GDK_KEY_' at the start.
// NOTE: Always pay attention to lower and upper names, for example 'space' vs 'Left'.
pub fn accelerators(accel: Accelerators) -> Vec<&'static str> {
    match accel {
        Accelerators::Quit => vec![action_accelerator_with_os_modifier!("Q")],
        Accelerators::FileChooser => vec![action_accelerator_with_os_modifier!("O")],
        Accelerators::TogglePlaylistVisibility => vec![action_accelerator_with_os_modifier!("B")],
        Accelerators::TogglePlayPause => vec!["space", "AudioPlay", action_accelerator_with_os_modifier!("space")],
        Accelerators::NextFrame => vec![action_accelerator_with_os_modifier!("<Alt>Right")],
        Accelerators::Previous => vec![action_accelerator_with_os_modifier!("Left"), "AudioPrev"],
        Accelerators::Next => vec![action_accelerator_with_os_modifier!("Right"), "AudioNext"],
        Accelerators::IncreaseVolume => vec![action_accelerator_with_os_modifier!("Up")],
        Accelerators::DecreaseVolume => vec![action_accelerator_with_os_modifier!("Down")],
        Accelerators::IncreaseSpeed => vec![action_accelerator_with_os_modifier!("<Alt>plus")],
        Accelerators::DecreaseSpeed => vec![action_accelerator_with_os_modifier!("<Alt>minus")],
        Accelerators::ResetSpeed => vec![action_accelerator_with_os_modifier!("<Alt>0")],
        Accelerators::FullScreen => vec![action_accelerator_with_os_modifier!("F")],
        Accelerators::DumpPipeline => vec![action_accelerator_with_os_modifier!("D")],
        Accelerators::Preferences => vec![action_accelerator_with_os_modifier!("comma")],
        Accelerators::VideoOffsets => vec![action_accelerator_with_os_modifier!("T")],
    }
}
