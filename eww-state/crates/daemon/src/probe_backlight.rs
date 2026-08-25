//! Screen brightness, from brightnessctl.

use eww_state_core::EventProbe;

use crate::proc;

pub fn sample() -> Option<EventProbe> {
    let out = proc::out("brightnessctl", &["-m"])?;
    Some(EventProbe::Backlight {
        percent: parse(&out)?,
    })
}

/// `brightnessctl -m` prints `device,class,current,percent,max`.
fn parse(out: &str) -> Option<u8> {
    let line = out.lines().next()?;
    let pct = line.split(',').nth(3)?.trim_end_matches('%');
    pct.parse::<u8>().ok()
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn brightness_percent() {
        assert_eq!(parse("amdgpu_bl1,backlight,133,52%,255"), Some(52));
        assert_eq!(parse(""), None);
    }
}
