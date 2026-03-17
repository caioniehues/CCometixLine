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
/// Used by Usage and ExtraUsage segments for dynamic pie-chart icons.
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
