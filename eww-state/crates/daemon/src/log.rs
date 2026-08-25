//! Where the log goes and what a line looks like.
//!
//! Nothing in this program prints. Every line carries a timestamp, the commit
//! that wrote it, a level and structured fields, because a line missing any of
//! those is the line missing from the report you are handed.

use std::fmt;
use std::fs::File;
use std::fs::OpenOptions;
use std::io;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use tracing::Event;
use tracing::Level;
use tracing::Subscriber;
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::FmtContext;
use tracing_subscriber::fmt::FormatEvent;
use tracing_subscriber::fmt::FormatFields;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::registry::LookupSpan;

/// The commit this binary was built from, baked in by `build.rs`.
pub const COMMIT: &str = env!("EWW_STATE_COMMIT");

/// Log file size at which the current file becomes `.1` and a fresh one starts.
/// A ring buffer on disk: bounded forever, always holding the recent history.
const MAX_BYTES: u64 = 2 * 1024 * 1024;

/// The daemon logs to a rotating file; it has no terminal to talk to.
pub fn init_daemon(path: PathBuf) {
    let writer = LogFile::new(path);
    tracing_subscriber::fmt()
        .event_format(Format)
        .with_max_level(max_level())
        .with_writer(writer)
        .init();
}

/// `EWW_STATE_LOG=debug` to see the quiet clock turns as well.
fn max_level() -> Level {
    match std::env::var("EWW_STATE_LOG").as_deref() {
        Ok("trace") => Level::TRACE,
        Ok("debug") => Level::DEBUG,
        Ok("warn") => Level::WARN,
        Ok("error") => Level::ERROR,
        _ => Level::INFO,
    }
}

/// The CLI logs to stderr, where the person who typed the command is looking.
pub fn init_client() {
    tracing_subscriber::fmt()
        .event_format(Bare)
        .with_writer(io::stderr)
        .init();
}

/// `<timestamp> <commit> <level> <fields>` — the daemon's line.
struct Format;

impl<S, N> FormatEvent<S, N> for Format
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        write!(
            writer,
            "{} {} {:<5} ",
            timestamp(),
            COMMIT,
            event.metadata().level()
        )?;
        ctx.field_format().format_fields(writer.by_ref(), event)?;
        writeln!(writer)
    }
}

/// `eww-state: <message>` — the CLI's line, which a person reads as a sentence
/// rather than as a record.
struct Bare;

impl<S, N> FormatEvent<S, N> for Bare
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        write!(writer, "eww-state: ")?;
        ctx.field_format().format_fields(writer.by_ref(), event)?;
        writeln!(writer)
    }
}

/// UTC, to the millisecond, RFC 3339.
///
/// Hand-rolled rather than pulling a date library in for one line: the whole of
/// it is the civil-from-days conversion below, which is exact for every day the
/// Unix epoch can express.
fn timestamp() -> String {
    let Ok(since) = SystemTime::now().duration_since(UNIX_EPOCH) else {
        return "0000-00-00T00:00:00.000Z".to_string();
    };
    let secs = since.as_secs();
    let millis = since.subsec_millis();
    let (days, rem) = (secs / 86_400, secs % 86_400);
    let (year, month, day) = civil_from_days(days as i64);
    let (h, m, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    format!("{year:04}-{month:02}-{day:02}T{h:02}:{m:02}:{s:02}.{millis:03}Z")
}

/// Howard Hinnant's `civil_from_days`, shifted to an era starting in March so
/// the leap day lands at the end of a year and needs no special case.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// A log file that never grows past [`MAX_BYTES`]: at the cap the current file
/// becomes `.1` and a fresh one starts, so the oldest turns fall off the end.
#[derive(Clone)]
struct LogFile {
    inner: Arc<Mutex<Inner>>,
}

struct Inner {
    path: PathBuf,
    file: Option<File>,
    size: u64,
}

impl LogFile {
    fn new(path: PathBuf) -> LogFile {
        LogFile {
            inner: Arc::new(Mutex::new(Inner {
                path,
                file: None,
                size: 0,
            })),
        }
    }
}

impl Inner {
    fn file(&mut self) -> io::Result<&mut File> {
        if self.file.is_none() {
            if let Some(parent) = self.path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)?;
            self.size = file.metadata().map(|m| m.len()).unwrap_or(0);
            self.file = Some(file);
        }
        let Some(file) = self.file.as_mut() else {
            unreachable!("one was just opened")
        };
        Ok(file)
    }

    fn rotate(&mut self) {
        self.file = None;
        self.size = 0;
        let previous = self.path.with_extension("1");
        let _ = std::fs::rename(&self.path, previous);
    }
}

impl Write for LogFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut inner = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if inner.size + buf.len() as u64 > MAX_BYTES {
            inner.rotate();
        }
        let written = inner.file()?.write(buf)?;
        inner.size += written as u64;
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        let mut inner = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        match inner.file.as_mut() {
            Some(file) => file.flush(),
            None => Ok(()),
        }
    }
}

impl<'a> MakeWriter<'a> for LogFile {
    type Writer = LogFile;

    fn make_writer(&'a self) -> LogFile {
        self.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::civil_from_days;

    #[test]
    fn days_convert_to_dates() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(59), (1970, 3, 1));
        // 2024 was a leap year: day 60 of it is the 29th of February.
        assert_eq!(civil_from_days(19_782), (2024, 2, 29));
        assert_eq!(civil_from_days(20_689), (2026, 8, 24));
    }
}
