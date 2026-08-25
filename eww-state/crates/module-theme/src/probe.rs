//! Reading the desktop's light/dark preference out of gsettings.

use crate::event::EventTheme;
use crate::model::Theme;

pub fn sample() -> EventTheme {
    EventTheme::Observed {
        theme: parse(
            &exec::out(
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
    use crate::model::Theme;

    #[test]
    fn dark_is_anything_saying_so() {
        assert_eq!(parse("'prefer-dark'"), Theme::Dark);
        assert_eq!(parse("'default'"), Theme::Light);
        assert_eq!(parse(""), Theme::Light);
    }
}
