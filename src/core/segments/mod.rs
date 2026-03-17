pub mod context_window;
pub mod cost;
pub mod directory;
pub mod effort;
pub mod extra_usage;
pub mod git;
pub mod model;
pub mod output_style;
pub mod session;
pub mod update;
pub mod usage;
pub mod usage_7d;

use crate::config::{InputData, SegmentId};
use std::collections::HashMap;

// New Segment trait for data collection only
pub trait Segment {
    fn collect(&self, input: &InputData) -> Option<SegmentData>;
    fn id(&self) -> SegmentId;
}

#[derive(Debug, Clone)]
pub struct SegmentData {
    pub primary: String,
    pub secondary: String,
    pub metadata: HashMap<String, String>,
}

/// Shared circle slice icon based on a 0.0–1.0 utilization fraction.
/// Used by ExtraUsage segment for dynamic pie-chart icons.
pub fn circle_icon_for_utilization(utilization: f64) -> &'static str {
    let percent = (utilization * 100.0) as u8;
    match percent {
        0..=12 => "\u{f0a9e}",
        13..=25 => "\u{f0a9f}",
        26..=37 => "\u{f0aa0}",
        38..=50 => "\u{f0aa1}",
        51..=62 => "\u{f0aa2}",
        63..=75 => "\u{f0aa3}",
        76..=87 => "\u{f0aa4}",
        _ => "\u{f0aa5}",
    }
}

/// FA hourglass icons for Usage (5h) segment — 4 fill levels.
pub fn hourglass_icon_for_utilization(utilization: f64) -> &'static str {
    let percent = (utilization * 100.0) as u8;
    match percent {
        0..=25 => "\u{f250}",  // nf-fa-hourglass_o (empty)
        26..=50 => "\u{f251}", // nf-fa-hourglass_1 (start/1-3)
        51..=75 => "\u{f252}", // nf-fa-hourglass_2 (half/2-3)
        _ => "\u{f253}",       // nf-fa-hourglass_3 (end/full)
    }
}

/// MD sand timer icons for Usage7d segment — 4 fill levels.
pub fn sand_timer_icon_for_utilization(utilization: f64) -> &'static str {
    let percent = (utilization * 100.0) as u8;
    match percent {
        0..=25 => "\u{f06ad}",  // nf-md-timer_sand_empty
        26..=50 => "\u{f051f}", // nf-md-timer_sand
        51..=75 => "\u{f078c}", // nf-md-timer_sand_full
        _ => "\u{f199f}",       // nf-md-timer_sand_complete
    }
}

// Re-export all segment types
pub use context_window::ContextWindowSegment;
pub use cost::CostSegment;
pub use directory::DirectorySegment;
pub use effort::EffortSegment;
pub use extra_usage::ExtraUsageSegment;
pub use git::GitSegment;
pub use model::ModelSegment;
pub use output_style::OutputStyleSegment;
pub use session::SessionSegment;
pub use update::UpdateSegment;
pub use usage::UsageSegment;
pub use usage_7d::Usage7dSegment;
