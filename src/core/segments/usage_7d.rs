use super::usage::UsageSegment;
use super::{Segment, SegmentData};
use crate::config::{InputData, SegmentId};
use std::collections::HashMap;

#[derive(Default)]
pub struct Usage7dSegment;

impl Usage7dSegment {
    pub fn new() -> Self {
        Self
    }
}

impl Segment for Usage7dSegment {
    fn collect(&self, _input: &InputData) -> Option<SegmentData> {
        let cache = UsageSegment::load_cache()?;

        let seven_day_util = cache.seven_day_utilization;
        let seven_day_percent = seven_day_util.round() as u8;
        let reset_time = UsageSegment::format_datetime(cache.resets_at.as_deref());
        let primary = format!("{}% @{}", seven_day_percent, reset_time);

        let dynamic_icon =
            super::sand_timer_icon_for_utilization(seven_day_util / 100.0).to_string();

        let mut metadata = HashMap::new();
        metadata.insert("dynamic_icon".to_string(), dynamic_icon);
        metadata.insert(
            "seven_day_utilization".to_string(),
            seven_day_util.to_string(),
        );

        Some(SegmentData {
            primary,
            secondary: String::new(),
            metadata,
        })
    }

    fn id(&self) -> SegmentId {
        SegmentId::Usage7d
    }
}
