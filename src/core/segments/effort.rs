use super::{Segment, SegmentData};
use crate::config::{InputData, SegmentId};
use std::collections::HashMap;

#[derive(Default)]
pub struct EffortSegment;

impl EffortSegment {
    pub fn new() -> Self {
        Self
    }

    fn get_effort_level(input: &InputData) -> String {
        // 1. Stdin JSON (mid-session /effort changes)
        if let Some(ref level) = input.effort_level {
            if !level.is_empty() {
                return level.clone();
            }
        }

        // 2. ~/.claude/settings.json → effortLevel
        if let Some(level) = Self::read_settings_effort() {
            return level;
        }

        // 3. CLAUDE_CODE_EFFORT_LEVEL env var
        if let Ok(level) = std::env::var("CLAUDE_CODE_EFFORT_LEVEL") {
            if !level.is_empty() {
                return level;
            }
        }

        // 4. Default
        "high".to_string()
    }

    fn read_settings_effort() -> Option<String> {
        let settings = crate::utils::settings::load_settings()?;
        settings
            .get("effortLevel")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
    }
}

impl Segment for EffortSegment {
    fn collect(&self, input: &InputData) -> Option<SegmentData> {
        let level = Self::get_effort_level(input);

        let mut metadata = HashMap::new();
        metadata.insert("effort_level".to_string(), level.clone());

        Some(SegmentData {
            primary: level,
            secondary: String::new(),
            metadata,
        })
    }

    fn id(&self) -> SegmentId {
        SegmentId::Effort
    }
}
