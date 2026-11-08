//! Reading the pack, the chart's ring, and what the machine is busy with.

use std::collections::BTreeMap;
use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use crate::event::EventBattery;
use crate::model::Battery;
use crate::model::HistPoint;
use crate::model::Proc;
use crate::model::Reading;

const SYSFS: &str = "/sys/class/power_supply/BAT0";

/// `/proc` reports CPU time in USER_HZ, which Linux fixes at 100 for the
/// userspace ABI whatever the kernel's own tick is. There is no portable way to
/// ask for it without libc, and it has not changed in the lifetime of `/proc`.
const USER_HZ: u64 = 100;

/// Read the pack and fold the reading into the ring.
pub fn sample(history: &str, span: u64, every: u64) -> EventBattery {
    let reading = read_pack();
    let now = unix_now();
    let point = HistPoint {
        at: now,
        percent: reading.battery.percent,
        charging: reading.battery.charging,
    };
    EventBattery::Observed {
        history: fold(history, point, span, every),
        reading,
    }
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default()
}

/// One pass over the uevent file, which is every field in one read — so the
/// percentage, the current and the charge cannot come from three different
/// instants and contradict each other.
fn read_pack() -> Reading {
    let text = std::fs::read_to_string(format!("{SYSFS}/uevent")).unwrap_or_default();
    let mut f: BTreeMap<&str, &str> = BTreeMap::new();
    for line in text.lines() {
        if let Some((k, v)) = line.split_once('=') {
            f.insert(k.trim(), v.trim());
        }
    }
    let num = |k: &str| f.get(k).and_then(|v| v.parse::<u64>().ok());
    let percent = num("POWER_SUPPLY_CAPACITY").unwrap_or_default().min(100) as u8;
    let status = f.get("POWER_SUPPLY_STATUS").copied().unwrap_or_default();
    // "Full" draws the same as charging: the cable is in.
    let charging = matches!(status, "Charging" | "Full");

    // Two families of kernel driver, and this machine is the charge one. A pack
    // reporting energy states it in µWh and µW directly; a pack reporting
    // charge states µAh and µA, and the power is the product with the voltage.
    let (milliwatts, seconds_left) = match (
        num("POWER_SUPPLY_ENERGY_NOW"),
        num("POWER_SUPPLY_POWER_NOW"),
    ) {
        (Some(energy), Some(power)) => {
            let full = num("POWER_SUPPLY_ENERGY_FULL").unwrap_or(energy);
            (power / 1_000, remaining(energy, full, power, charging))
        }
        _ => {
            let charge = num("POWER_SUPPLY_CHARGE_NOW").unwrap_or_default();
            let full = num("POWER_SUPPLY_CHARGE_FULL").unwrap_or(charge);
            let current = num("POWER_SUPPLY_CURRENT_NOW").unwrap_or_default();
            let volts = num("POWER_SUPPLY_VOLTAGE_NOW").unwrap_or_default();
            // µV * µA is 1e-12 W, so 1e9 of them is a milliwatt. Both are under
            // 2e7 on any pack, so the product cannot overflow a u64.
            (
                volts.saturating_mul(current) / 1_000_000_000,
                remaining(charge, full, current, charging),
            )
        }
    };

    Reading {
        battery: Battery { percent, charging },
        milliwatts: milliwatts.min(u32::MAX as u64) as u32,
        seconds_left,
        health: f
            .get("POWER_SUPPLY_HEALTH")
            .copied()
            .unwrap_or("Unknown")
            .to_string(),
        cycles: num("POWER_SUPPLY_CYCLE_COUNT").unwrap_or_default() as u32,
    }
}

/// Seconds to empty, or to full on the cable. The same division `upower` does:
/// what is left over what is leaving. Zero rate has no answer, and saying so is
/// better than a number that is really a division by nothing.
fn remaining(now: u64, full: u64, rate: u64, charging: bool) -> Option<u32> {
    if rate == 0 {
        return None;
    }
    let left = if charging {
        full.saturating_sub(now)
    } else {
        now
    };
    Some((left.saturating_mul(3_600) / rate).min(u32::MAX as u64) as u32)
}

/// Add a point to the ring, drop what has aged out, and hand back what is left.
///
/// The file is the ring. Nothing is kept in memory beside it, so a daemon that
/// restarts picks up the chart where it left off rather than drawing a line
/// that starts at the moment it came up.
fn fold(path: &str, point: HistPoint, span: u64, every: u64) -> Vec<HistPoint> {
    let mut points = read_history(path);
    let store = match points.last() {
        // A point every fifteen seconds would be ten thousand lines for one
        // screen's worth of line. A move in the percentage is always worth
        // keeping, though: it is the only thing the chart actually draws.
        Some(last) => {
            point.at.saturating_sub(last.at) >= every
                || last.percent != point.percent
                || last.charging != point.charging
        }
        None => true,
    };
    if !store {
        return points;
    }
    points.push(point);
    let cutoff = point.at.saturating_sub(span);
    points.retain(|p| p.at >= cutoff);
    write_history(path, &points);
    points
}

fn read_history(path: &str) -> Vec<HistPoint> {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let mut points: Vec<HistPoint> = text
        .lines()
        .filter_map(|line| {
            let mut it = line.split('\t');
            Some(HistPoint {
                at: it.next()?.parse().ok()?,
                percent: it.next()?.parse().ok()?,
                charging: it.next()? == "1",
            })
        })
        .collect();
    // A clock that went backwards — an NTP step, a dual boot — would otherwise
    // draw the line doubling back on itself.
    points.sort_by_key(|p| p.at);
    points
}

/// Written whole through a temporary file, so a daemon killed mid-write leaves
/// the previous ring rather than half a line the parser would drop.
fn write_history(path: &str, points: &[HistPoint]) {
    let mut out = String::with_capacity(points.len() * 16);
    for p in points {
        out.push_str(&format!(
            "{}\t{}\t{}\n",
            p.at,
            p.percent,
            u8::from(p.charging)
        ));
    }
    let tmp = format!("{path}.tmp");
    if std::fs::write(&tmp, out).is_ok() {
        let _ = std::fs::rename(&tmp, path);
    }
}

/// Rank what the machine is spending its CPU on, by app.
///
/// There is no per-process power meter to read: `intel-rapl`'s `energy_uj` is
/// root-only and this daemon is not root. So the ranking is CPU time, measured
/// over a window.
///
/// Attribution is by **window**, not by process group. A process group would be
/// the obvious grouping and is wrong here: the compositor spawns Firefox and
/// the terminal itself, so both inherit the login session's group and every app
/// on the machine collapses into one row called `start-hyprland`. Walking up
/// the parent chain to the nearest process the compositor says owns a window
/// gives the grouping a person actually means — Firefox's content processes
/// land on Firefox, and a build running in a terminal lands on the terminal,
/// which is the window they would close to stop it.
///
/// A process with no windowed ancestor is left out. The list is titled by app
/// and drawn with app icons, and a row for `wireplumber` is neither. The cost
/// is that the shares do not sum to the machine's whole load, which is why the
/// panel never presents them as if they did.
pub fn procs(
    window_ms: u32,
    top: usize,
    icons: &[(String, String)],
    owners: &[(u32, String)],
    fallback: &str,
) -> Vec<Proc> {
    let first = snapshot();
    std::thread::sleep(Duration::from_millis(window_ms.into()));
    let second = snapshot();

    let busy = second.busy.saturating_sub(first.busy);
    if busy == 0 {
        return Vec::new();
    }
    let boot_secs = uptime_secs();

    // Several windows of one app are one row: whoever opened three terminals
    // thinks of them as the terminal, and three rows with the same name and
    // icon is a list nobody can read.
    let owner_class: BTreeMap<u32, &str> = owners
        .iter()
        .map(|(pid, class)| (*pid, class.as_str()))
        .collect();
    let mut named: BTreeMap<&str, (u64, u64)> = BTreeMap::new();
    for (pid, after) in &second.procs {
        let Some(before) = first.procs.get(pid) else {
            continue;
        };
        let spent = after.ticks.saturating_sub(before.ticks);
        if spent == 0 {
            continue;
        }
        let Some((owner, class)) = owning_window(*pid, &second.procs, &owner_class) else {
            continue;
        };
        let started = second
            .procs
            .get(&owner)
            .map_or(0, |p| boot_secs.saturating_sub(p.started / USER_HZ));
        let row = named.entry(class).or_default();
        row.0 = row.0.saturating_add(spent);
        // The oldest window of the app, which is how long the app has been up
        // rather than how long its most recent window has.
        row.1 = row.1.max(started);
    }

    let mut rows: Vec<Proc> = named
        .into_iter()
        .map(|(class, (spent, uptime))| Proc {
            icon: icon_for(class, icons).unwrap_or_else(|| fallback.to_string()),
            share: (spent.saturating_mul(100) / busy).min(100) as u8,
            name: display_name(class, icons),
            uptime,
        })
        .collect();
    // Descending by share, then by name so a tie does not reorder the list on
    // every sample — a row swapping places with its neighbour twice a second
    // reads as a bug even when both numbers are right.
    rows.sort_by(|a, b| b.share.cmp(&a.share).then_with(|| a.name.cmp(&b.name)));
    rows.retain(|p| p.share > 0);
    rows.truncate(top);
    rows
}

#[derive(Default)]
struct Snapshot {
    busy: u64,
    procs: BTreeMap<u32, ProcStat>,
}

fn snapshot() -> Snapshot {
    Snapshot {
        busy: busy_ticks(),
        procs: walk_procs(),
    }
}

/// How far up a parent chain to look before giving up.
///
/// A real chain is three or four deep. The cap is not about depth, it is about
/// termination: `/proc` is read one process at a time and a parent can exit and
/// its pid be reused between two of those reads, which is enough to make a
/// cycle out of what was never one.
const MAX_ANCESTRY: usize = 12;

/// The nearest process at or above this one that the compositor says owns a
/// window, and what that window's app is called.
fn owning_window<'a>(
    pid: u32,
    procs: &BTreeMap<u32, ProcStat>,
    owners: &BTreeMap<u32, &'a str>,
) -> Option<(u32, &'a str)> {
    let mut at = pid;
    for _ in 0..MAX_ANCESTRY {
        if let Some(class) = owners.get(&at) {
            return Some((at, class));
        }
        let parent = procs.get(&at)?.ppid;
        if parent <= 1 {
            return None;
        }
        at = parent;
    }
    None
}

/// Busy jiffies across every CPU: everything the kernel counts except idle and
/// iowait. Idle is excluded because the panel ranks what is spending energy,
/// and against a mostly-idle machine every share would round to zero.
fn busy_ticks() -> u64 {
    let stat = std::fs::read_to_string("/proc/stat").unwrap_or_default();
    let Some(line) = stat.lines().next() else {
        return 0;
    };
    let fields: Vec<u64> = line
        .split_whitespace()
        .skip(1)
        .filter_map(|f| f.parse().ok())
        .collect();
    let total: u64 = fields.iter().sum();
    // user nice system idle iowait ...
    let idle: u64 = fields.iter().skip(3).take(2).sum();
    total.saturating_sub(idle)
}

fn uptime_secs() -> u64 {
    std::fs::read_to_string("/proc/uptime")
        .ok()
        .and_then(|s| s.split_whitespace().next()?.parse::<f64>().ok())
        .unwrap_or_default() as u64
}

fn walk_procs() -> BTreeMap<u32, ProcStat> {
    let mut procs: BTreeMap<u32, ProcStat> = BTreeMap::new();
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return procs;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(pid) = name.to_string_lossy().parse::<u32>().ok() else {
            continue;
        };
        if let Some(p) = read_proc_stat(pid) {
            procs.insert(pid, p);
        }
    }
    procs
}

struct ProcStat {
    ppid: u32,
    ticks: u64,
    started: u64,
}

/// `/proc/<pid>/stat`, which cannot be split on whitespace from the left: the
/// command is in parentheses and may contain both spaces and parentheses of its
/// own. Everything is counted from the LAST `)` instead.
fn read_proc_stat(pid: u32) -> Option<ProcStat> {
    let text = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let close = text.rfind(')')?;
    let rest: Vec<&str> = text.get(close + 1..)?.split_whitespace().collect();
    // Offsets into what follows the command, so the numbering is that of
    // proc(5) minus the two fields before it: ppid is 4, utime 14, stime 15,
    // starttime 22.
    let at = |i: usize| rest.get(i).and_then(|v| v.parse::<u64>().ok());
    Some(ProcStat {
        ppid: at(1)? as u32,
        ticks: at(11)?.saturating_add(at(12)?),
        started: at(19)?,
    })
}

/// A dock entry's icon for this window class, if any entry claims it.
///
/// Matched loosely in both directions because the two names are written by
/// different people: the dock calls it `signal`, the window calls itself
/// `signal-desktop`.
fn icon_for(class: &str, icons: &[(String, String)]) -> Option<String> {
    matching(class, icons).map(|(_, icon)| icon.clone())
}

fn matching<'a>(class: &str, icons: &'a [(String, String)]) -> Option<&'a (String, String)> {
    let class = class.to_lowercase();
    icons.iter().find(|(id, _)| {
        let id = id.to_lowercase();
        !id.is_empty() && (class.starts_with(&id) || id.starts_with(&class))
    })
}

/// What the row is called. The dock's name wins where there is one, because it
/// is the name a person chose: `signal`, not `signal-desktop`.
fn display_name(class: &str, icons: &[(String, String)]) -> String {
    let name = matching(class, icons).map_or(class, |(id, _)| id.as_str());
    let mut chars = name.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}
