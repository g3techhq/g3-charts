//! The maths behind the charts, with no Dioxus in it: scales, "nice" ticks,
//! time axes, arc paths, LTTB downsampling, nearest-point search and value
//! formatting. Public, so an app can label its own axes the same way.

mod arc;
mod format;
mod lttb;
mod nearest;
mod path;
mod scale;
mod ticks;
mod time;

pub use arc::{Dash, angle_at, angles, centroid, dash, polar, sector_path};
pub use format::ValueFormat;
pub use lttb::lttb;
pub use nearest::{nearest_by, nearest_index};
pub use path::{PathBuilder, fmt};
pub use scale::{BandScale, LinearScale};
pub use ticks::{linear_ticks, nice_domain, nice_step, step_decimals};
pub use time::{
    CivilDate, DAY, HOUR, MINUTE, SECOND, TimeInterval, civil_from_days, date_label,
    days_from_civil, time_interval, time_label, time_ticks,
};
