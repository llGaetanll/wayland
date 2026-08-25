//! The desktop's light/dark preference.

use eww_state_core::EventProbe;
use eww_state_core::Theme;

use crate::proc;

pub fn sample() -> EventProbe {
    EventProbe::Theme {
        theme: parse(
            &proc::out(
                "gsettings",
                &["get", "org.gnome.desktop.interface", "color-scheme"],
            )
            .unwrap_or_default(),
        ),
    }
}

fn parse(out: &str) -> Theme {
    if out.to_ascii_lowercase().contains("dark") {
        Theme::Dark
    } else {
        Theme::Light
    }
}

#[cfg(test)]
mod tests {
    use super::parse;
    use eww_state_core::Theme;

    #[test]
    fn theme_detection() {
        assert_eq!(parse("'prefer-dark'"), Theme::Dark);
        assert_eq!(parse("'default'"), Theme::Light);
    }
}
