//! Host-level status-line data: machine name, RAM, and disk.
//!
//! The probing functions wrap `sysinfo`; everything above them
//! (unit selection, mount matching) is pure so it can be tested
//! without a particular machine shape.

use std::cell::OnceCell;
use std::path::{Path, PathBuf};

use sysinfo::{DiskRefreshKind, Disks, MemoryRefreshKind, RefreshKind, System};

use super::theme::{RESET, label, usage_color};
use super::widget::Widget;

/// A used-of-total byte quantity: RAM, or a mounted filesystem.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Usage {
    pub used: u64,
    pub total: u64,
}

impl Usage {
    pub fn percentage(self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            self.used as f64 * 100.0 / self.total as f64
        }
    }

    /// Render as `used/totalUNIT`, e.g. `12.4/31.3G` or `210/468G`.
    /// Both figures share the unit chosen for `total`, so the pair
    /// reads as a single ratio.
    pub fn render(self) -> String {
        let unit = Unit::for_bytes(self.total);
        format!(
            "{}/{}{}",
            unit.scale(self.used),
            unit.scale(self.total),
            unit.suffix()
        )
    }
}

/// Binary (1024-based) magnitude used to render a byte count.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Unit {
    Bytes,
    Kilo,
    Mega,
    Giga,
    Tera,
}

const KIB: u64 = 1024;
const MIB: u64 = KIB * 1024;
const GIB: u64 = MIB * 1024;
const TIB: u64 = GIB * 1024;

impl Unit {
    fn for_bytes(bytes: u64) -> Self {
        if bytes >= TIB {
            Self::Tera
        } else if bytes >= GIB {
            Self::Giga
        } else if bytes >= MIB {
            Self::Mega
        } else if bytes >= KIB {
            Self::Kilo
        } else {
            Self::Bytes
        }
    }

    fn divisor(self) -> u64 {
        match self {
            Self::Bytes => 1,
            Self::Kilo => KIB,
            Self::Mega => MIB,
            Self::Giga => GIB,
            Self::Tera => TIB,
        }
    }

    fn suffix(self) -> &'static str {
        match self {
            Self::Bytes => "B",
            Self::Kilo => "K",
            Self::Mega => "M",
            Self::Giga => "G",
            Self::Tera => "T",
        }
    }

    /// One decimal below 100, none above, so the pair stays narrow
    /// without losing resolution on small values.
    fn scale(self, bytes: u64) -> String {
        if self == Self::Bytes {
            return bytes.to_string();
        }
        let value = bytes as f64 / self.divisor() as f64;
        if value < 100.0 {
            format!("{value:.1}")
        } else {
            format!("{value:.0}")
        }
    }
}

/// A mounted filesystem, reduced to what the widget needs.
#[derive(Clone, Debug, PartialEq)]
pub struct Mount {
    pub mount_point: PathBuf,
    pub usage: Usage,
}

/// Lazily-probed host information, shared across the `host`, `ram`,
/// and `disk` widgets so a single render probes the system at most
/// once per kind of data.
pub struct SystemContext {
    /// The directory whose filesystem `disk` reports on, resolved
    /// once when the render starts.
    dir: PathBuf,
    host_name: OnceCell<Option<String>>,
    memory: OnceCell<Option<Usage>>,
    mounts: OnceCell<Vec<Mount>>,
}

impl SystemContext {
    pub fn new(dir: PathBuf) -> Self {
        Self {
            dir,
            host_name: OnceCell::new(),
            memory: OnceCell::new(),
            mounts: OnceCell::new(),
        }
    }

    fn host_name(&self) -> Option<&str> {
        self.host_name.get_or_init(host_name).as_deref()
    }

    fn memory(&self) -> Option<Usage> {
        *self.memory.get_or_init(memory_usage)
    }

    fn disk(&self) -> Option<Usage> {
        let mounts = self.mounts.get_or_init(mounts);
        mount_for(mounts, &self.dir).map(|m| m.usage)
    }
}

/// `ram 12.4/31.3G` — the figure colored by how full it is.
fn render_usage(lbl: &str, usage: Usage) -> String {
    let color = usage_color(usage.percentage());
    format!("{} {color}{}{RESET}", label(lbl), usage.render())
}

/// Render a host-backed widget, or `None` when the name belongs to
/// another family or the platform reports nothing.
pub fn render(widget: &Widget, sys: &SystemContext) -> Option<String> {
    match widget {
        Widget::Host => {
            sys.host_name().map(|h| format!("{} {h}", label("host")))
        }
        Widget::Ram => sys.memory().map(|usage| render_usage("ram", usage)),
        Widget::Disk => sys.disk().map(|usage| render_usage("disk", usage)),
        _ => None,
    }
}

/// The machine's short host name (domain suffix stripped).
pub fn host_name() -> Option<String> {
    let name = System::host_name()?;
    let short = name.split('.').next().unwrap_or(&name).trim().to_string();
    if short.is_empty() { None } else { Some(short) }
}

/// Physical RAM in use versus installed. `None` when the platform
/// reports no memory at all (unsupported target).
pub fn memory_usage() -> Option<Usage> {
    let sys = System::new_with_specifics(
        RefreshKind::nothing()
            .with_memory(MemoryRefreshKind::nothing().with_ram()),
    );
    let total = sys.total_memory();
    if total == 0 {
        return None;
    }
    Some(Usage {
        used: sys.used_memory().min(total),
        total,
    })
}

/// Every mounted filesystem that reports a non-zero size.
pub fn mounts() -> Vec<Mount> {
    let disks = Disks::new_with_refreshed_list_specifics(
        DiskRefreshKind::nothing().with_storage(),
    );
    disks
        .list()
        .iter()
        .filter(|d| d.total_space() > 0)
        .map(|d| {
            let total = d.total_space();
            Mount {
                mount_point: d.mount_point().to_path_buf(),
                usage: Usage {
                    used: total.saturating_sub(d.available_space()),
                    total,
                },
            }
        })
        .collect()
}

/// The mount that `path` lives on: the longest mount point that is a
/// path-prefix of it. Falls back to the root-most mount when nothing
/// matches, so the widget still has something to show on hosts with
/// unusual mount tables.
pub fn mount_for<'a>(mounts: &'a [Mount], path: &Path) -> Option<&'a Mount> {
    let key = path_key(path);
    mounts
        .iter()
        .filter(|m| key.starts_with(path_key(&m.mount_point)))
        .max_by_key(|m| m.mount_point.components().count())
        .or_else(|| {
            mounts
                .iter()
                .min_by_key(|m| m.mount_point.components().count())
        })
}

/// Comparison form of a path. Windows paths are case-insensitive, so
/// `C:\Users` must match a mount reported as `C:\`.
fn path_key(path: &Path) -> PathBuf {
    if cfg!(windows) {
        PathBuf::from(path.to_string_lossy().to_lowercase())
    } else {
        path.to_path_buf()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mount(point: &str, used: u64, total: u64) -> Mount {
        Mount {
            mount_point: PathBuf::from(point),
            usage: Usage { used, total },
        }
    }

    #[test]
    fn unit_picks_binary_magnitude() {
        assert_eq!(Unit::for_bytes(0), Unit::Bytes);
        assert_eq!(Unit::for_bytes(1023), Unit::Bytes);
        assert_eq!(Unit::for_bytes(KIB), Unit::Kilo);
        assert_eq!(Unit::for_bytes(MIB - 1), Unit::Kilo);
        assert_eq!(Unit::for_bytes(MIB), Unit::Mega);
        assert_eq!(Unit::for_bytes(GIB), Unit::Giga);
        assert_eq!(Unit::for_bytes(TIB), Unit::Tera);
    }

    #[test]
    fn render_uses_one_decimal_below_hundred() {
        let usage = Usage {
            used: 12 * GIB + GIB / 2,
            total: 32 * GIB,
        };
        assert_eq!(usage.render(), "12.5/32.0G");
    }

    #[test]
    fn render_drops_decimals_above_hundred() {
        let usage = Usage {
            used: 210 * GIB,
            total: 468 * GIB,
        };
        assert_eq!(usage.render(), "210/468G");
    }

    #[test]
    fn render_shares_the_total_unit() {
        // A small used value keeps the total's unit rather than
        // dropping to its own, so the ratio stays readable.
        let usage = Usage {
            used: 512 * MIB,
            total: 8 * GIB,
        };
        assert_eq!(usage.render(), "0.5/8.0G");
    }

    #[test]
    fn render_raw_bytes_without_decimals() {
        let usage = Usage {
            used: 12,
            total: 900,
        };
        assert_eq!(usage.render(), "12/900B");
    }

    #[test]
    fn percentage_of_empty_total_is_zero() {
        assert_eq!(Usage { used: 0, total: 0 }.percentage(), 0.0);
    }

    #[test]
    fn percentage_is_used_over_total() {
        let usage = Usage {
            used: 25,
            total: 200,
        };
        assert_eq!(usage.percentage(), 12.5);
    }

    #[test]
    fn mount_for_picks_longest_prefix() {
        let mounts = vec![
            mount("/", 1, 100),
            mount("/home", 2, 200),
            mount("/home/vagrant/data", 3, 300),
        ];
        let found = mount_for(&mounts, Path::new("/home/vagrant/project"))
            .expect("should match");
        assert_eq!(found.mount_point, PathBuf::from("/home"));
    }

    #[test]
    fn mount_for_matches_whole_components_only() {
        // "/homework" must not be attributed to the "/home" mount.
        let mounts = vec![mount("/", 1, 100), mount("/home", 2, 200)];
        let found =
            mount_for(&mounts, Path::new("/homework/x")).expect("should match");
        assert_eq!(found.mount_point, PathBuf::from("/"));
    }

    #[test]
    fn mount_for_falls_back_to_root_most_mount() {
        let mounts = vec![mount("/data", 1, 100), mount("/data/deep", 2, 200)];
        let found =
            mount_for(&mounts, Path::new("/elsewhere")).expect("should match");
        assert_eq!(found.mount_point, PathBuf::from("/data"));
    }

    #[test]
    fn mount_for_empty_list_is_none() {
        assert!(mount_for(&[], Path::new("/")).is_none());
    }

    #[test]
    fn memory_usage_is_plausible() {
        let usage = memory_usage().expect("host should report memory");
        assert!(usage.total > 0);
        assert!(usage.used <= usage.total);
    }

    #[test]
    fn host_name_is_non_empty() {
        let name = host_name().expect("host should have a name");
        assert!(!name.is_empty());
        assert!(!name.contains('.'));
    }

    #[test]
    fn render_usage_colors_by_fullness() {
        let usage = Usage {
            used: 1,
            total: 100,
        };
        let out = render_usage("ram", usage);
        assert!(out.contains("ram"));
        assert!(out.contains("1/100B"));
        assert!(out.contains(super::super::theme::GREEN));
    }

    #[test]
    fn host_widgets_render_on_this_machine() {
        let sys = SystemContext::new(PathBuf::from("."));
        for widget in [Widget::Host, Widget::Ram, Widget::Disk] {
            let out = render(&widget, &sys)
                .unwrap_or_else(|| panic!("{widget} should render"));
            assert!(!out.is_empty());
        }
    }

    #[test]
    fn foreign_widget_name_is_declined() {
        let sys = SystemContext::new(PathBuf::from("."));
        // A widget owned by another family is declined.
        assert_eq!(render(&Widget::Model, &sys), None);
    }

    #[test]
    fn mounts_report_sized_filesystems() {
        for m in mounts() {
            assert!(m.usage.total > 0);
            assert!(m.usage.used <= m.usage.total);
        }
    }
}
