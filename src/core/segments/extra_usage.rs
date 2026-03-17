use super::usage::UsageSegment;
use super::{Segment, SegmentData};
use crate::config::{InputData, SegmentId};
use std::collections::HashMap;

#[derive(Default)]
pub struct ExtraUsageSegment;

impl ExtraUsageSegment {
    pub fn new() -> Self {
        Self
    }
}

impl Segment for ExtraUsageSegment {
    fn collect(&self, _input: &InputData) -> Option<SegmentData> {
        let cache = UsageSegment::load_cache()?;

        if !cache.extra_usage_enabled {
            return None;
        }

        let used_dollars = cache.extra_usage_used_credits / 100.0;
        let limit_dollars = cache.extra_usage_monthly_limit / 100.0;
        let utilization_fraction = cache.extra_usage_utilization / 100.0;

        let primary = format!("${:.2}/${:.2}", used_dollars, limit_dollars);
        let dynamic_icon = super::circle_icon_for_utilization(utilization_fraction).to_string();

        let mut metadata = HashMap::new();
        metadata.insert("dynamic_icon".to_string(), dynamic_icon);
        metadata.insert(
            "extra_usage_utilization".to_string(),
            cache.extra_usage_utilization.to_string(),
        );
        metadata.insert("used_credits".to_string(), format!("{:.2}", used_dollars));
        metadata.insert("monthly_limit".to_string(), format!("{:.2}", limit_dollars));

        Some(SegmentData {
            primary,
            secondary: String::new(),
            metadata,
        })
    }

    fn id(&self) -> SegmentId {
        SegmentId::ExtraUsage
    }
}
