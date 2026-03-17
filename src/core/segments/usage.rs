use super::{Segment, SegmentData};
use crate::config::{InputData, SegmentId};
use crate::utils::credentials;
use chrono::{DateTime, Datelike, Duration, Local, Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
struct ApiUsageResponse {
    five_hour: UsagePeriod,
    seven_day: UsagePeriod,
    #[serde(default)]
    extra_usage: Option<ExtraUsagePeriod>,
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct ExtraUsagePeriod {
    #[serde(default)]
    pub is_enabled: bool,
    #[serde(default)]
    pub utilization: f64,
    #[serde(default)]
    pub used_credits: f64,
    #[serde(default)]
    pub monthly_limit: f64,
}

#[derive(Debug, Deserialize)]
struct UsagePeriod {
    utilization: f64,
    resets_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ApiUsageCache {
    pub five_hour_utilization: f64,
    pub seven_day_utilization: f64,
    pub resets_at: Option<String>,
    pub cached_at: String,
    #[serde(default)]
    pub extra_usage_enabled: bool,
    #[serde(default)]
    pub extra_usage_utilization: f64,
    #[serde(default)]
    pub extra_usage_used_credits: f64,
    #[serde(default)]
    pub extra_usage_monthly_limit: f64,
}

#[derive(Default)]
pub struct UsageSegment;

impl UsageSegment {
    pub fn new() -> Self {
        Self
    }

    fn format_reset_time(reset_time_str: Option<&str>) -> String {
        if let Some(time_str) = reset_time_str {
            if let Ok(dt) = DateTime::parse_from_rfc3339(time_str) {
                let mut local_dt = dt.with_timezone(&Local);
                if local_dt.minute() > 45 {
                    local_dt += Duration::hours(1);
                }
                return format!(
                    "{}-{}-{}",
                    local_dt.month(),
                    local_dt.day(),
                    local_dt.hour()
                );
            }
        }
        "?".to_string()
    }

    pub(crate) fn get_cache_path() -> Option<std::path::PathBuf> {
        let home = dirs::home_dir()?;
        Some(
            home.join(".claude")
                .join("ccline")
                .join(".api_usage_cache.json"),
        )
    }

    pub(crate) fn load_cache() -> Option<ApiUsageCache> {
        let cache_path = Self::get_cache_path()?;
        let content = std::fs::read_to_string(&cache_path).ok()?;
        serde_json::from_str(&content).ok()
    }

    fn save_cache(&self, cache: &ApiUsageCache) {
        if let Some(cache_path) = Self::get_cache_path() {
            if let Some(parent) = cache_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Ok(json) = serde_json::to_string_pretty(cache) {
                let _ = std::fs::write(&cache_path, json);
            }
        }
    }

    pub(crate) fn is_cache_valid(cache: &ApiUsageCache, cache_duration: u64) -> bool {
        if let Ok(cached_at) = DateTime::parse_from_rfc3339(&cache.cached_at) {
            let now = Utc::now();
            let elapsed = now.signed_duration_since(cached_at.with_timezone(&Utc));
            elapsed.num_seconds() < cache_duration as i64
        } else {
            false
        }
    }

    fn get_claude_code_version() -> String {
        use std::process::Command;

        let output = Command::new("npm")
            .args(["view", "@anthropic-ai/claude-code", "version"])
            .output();

        match output {
            Ok(output) if output.status.success() => {
                let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !version.is_empty() {
                    return format!("claude-code/{}", version);
                }
            }
            _ => {}
        }

        "claude-code".to_string()
    }

    fn get_proxy_from_settings() -> Option<String> {
        let settings = crate::utils::settings::load_settings()?;

        // Try HTTPS_PROXY first, then HTTP_PROXY
        settings
            .get("env")?
            .get("HTTPS_PROXY")
            .or_else(|| settings.get("env")?.get("HTTP_PROXY"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    fn fetch_api_usage(
        &self,
        api_base_url: &str,
        token: &str,
        timeout_secs: u64,
    ) -> Option<ApiUsageResponse> {
        let url = format!("{}/api/oauth/usage", api_base_url);
        let user_agent = Self::get_claude_code_version();

        let agent = if let Some(proxy_url) = Self::get_proxy_from_settings() {
            if let Ok(proxy) = ureq::Proxy::new(&proxy_url) {
                ureq::Agent::config_builder()
                    .proxy(Some(proxy))
                    .build()
                    .new_agent()
            } else {
                ureq::Agent::new_with_defaults()
            }
        } else {
            ureq::Agent::new_with_defaults()
        };

        let response = agent
            .get(&url)
            .header("Authorization", &format!("Bearer {}", token))
            .header("anthropic-beta", "oauth-2025-04-20")
            .header("User-Agent", &user_agent)
            .config()
            .timeout_global(Some(std::time::Duration::from_secs(timeout_secs)))
            .build()
            .call()
            .ok()?;

        response.into_body().read_json().ok()
    }
}

impl Segment for UsageSegment {
    fn collect(&self, _input: &InputData) -> Option<SegmentData> {
        let token = credentials::get_oauth_token()?;

        // Load config from file to get segment options
        let config = crate::config::Config::load().ok()?;
        let segment_config = config.segments.iter().find(|s| s.id == SegmentId::Usage);

        let api_base_url = segment_config
            .and_then(|sc| sc.options.get("api_base_url"))
            .and_then(|v| v.as_str())
            .unwrap_or("https://api.anthropic.com");

        let cache_duration = segment_config
            .and_then(|sc| sc.options.get("cache_duration"))
            .and_then(|v| v.as_u64())
            .unwrap_or(300);

        let timeout = segment_config
            .and_then(|sc| sc.options.get("timeout"))
            .and_then(|v| v.as_u64())
            .unwrap_or(2);

        let cached_data = Self::load_cache();
        let use_cached = cached_data
            .as_ref()
            .map(|cache| Self::is_cache_valid(cache, cache_duration))
            .unwrap_or(false);

        let (five_hour_util, seven_day_util, resets_at) = if use_cached {
            let cache = cached_data.unwrap();
            (
                cache.five_hour_utilization,
                cache.seven_day_utilization,
                cache.resets_at,
            )
        } else {
            match self.fetch_api_usage(api_base_url, &token, timeout) {
                Some(response) => {
                    let extra = response.extra_usage.as_ref();
                    let cache = ApiUsageCache {
                        five_hour_utilization: response.five_hour.utilization,
                        seven_day_utilization: response.seven_day.utilization,
                        resets_at: response.seven_day.resets_at.clone(),
                        cached_at: Utc::now().to_rfc3339(),
                        extra_usage_enabled: extra.is_some_and(|e| e.is_enabled),
                        extra_usage_utilization: extra.map_or(0.0, |e| e.utilization),
                        extra_usage_used_credits: extra.map_or(0.0, |e| e.used_credits),
                        extra_usage_monthly_limit: extra.map_or(0.0, |e| e.monthly_limit),
                    };
                    self.save_cache(&cache);
                    (
                        response.five_hour.utilization,
                        response.seven_day.utilization,
                        response.seven_day.resets_at,
                    )
                }
                None => {
                    if let Some(cache) = cached_data {
                        (
                            cache.five_hour_utilization,
                            cache.seven_day_utilization,
                            cache.resets_at,
                        )
                    } else {
                        return None;
                    }
                }
            }
        };

        let dynamic_icon = super::circle_icon_for_utilization(seven_day_util / 100.0).to_string();
        let five_hour_percent = five_hour_util.round() as u8;
        let primary = format!("{}%", five_hour_percent);
        let secondary = format!("· {}", Self::format_reset_time(resets_at.as_deref()));

        let mut metadata = HashMap::new();
        metadata.insert("dynamic_icon".to_string(), dynamic_icon);
        metadata.insert(
            "five_hour_utilization".to_string(),
            five_hour_util.to_string(),
        );
        metadata.insert(
            "seven_day_utilization".to_string(),
            seven_day_util.to_string(),
        );

        Some(SegmentData {
            primary,
            secondary,
            metadata,
        })
    }

    fn id(&self) -> SegmentId {
        SegmentId::Usage
    }
}
