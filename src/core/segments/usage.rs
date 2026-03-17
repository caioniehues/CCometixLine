use super::{Segment, SegmentData};
use crate::config::{InputData, SegmentId};
use crate::utils::credentials;
use chrono::{DateTime, Datelike, Local, Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize, Serialize)]
struct ApiUsageResponse {
    five_hour: UsagePeriod,
    seven_day: UsagePeriod,
    #[serde(default)]
    extra_usage: Option<ExtraUsagePeriod>,
}

/// Deserialize helper: treat null as default (0.0 for f64, false for bool).
fn null_as_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Default + Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(|opt| opt.unwrap_or_default())
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct ExtraUsagePeriod {
    #[serde(default, deserialize_with = "null_as_default")]
    pub is_enabled: bool,
    #[serde(default, deserialize_with = "null_as_default")]
    pub utilization: f64,
    #[serde(default, deserialize_with = "null_as_default")]
    pub used_credits: f64,
    #[serde(default, deserialize_with = "null_as_default")]
    pub monthly_limit: f64,
}

#[derive(Debug, Deserialize, Serialize)]
struct UsagePeriod {
    utilization: f64,
    resets_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ApiUsageCache {
    pub five_hour_utilization: f64,
    pub seven_day_utilization: f64,
    pub resets_at: Option<String>,
    #[serde(default)]
    pub five_hour_resets_at: Option<String>,
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

    fn parse_to_local(reset_time_str: Option<&str>) -> Option<DateTime<Local>> {
        let time_str = reset_time_str?;
        DateTime::parse_from_rfc3339(time_str)
            .ok()
            .map(|dt| dt.with_timezone(&Local))
    }

    pub(crate) fn format_time_only(reset_time_str: Option<&str>) -> String {
        match Self::parse_to_local(reset_time_str) {
            Some(dt) => format!("{:02}:{:02}", dt.hour(), dt.minute()),
            None => "?".to_string(),
        }
    }

    pub(crate) fn format_datetime(reset_time_str: Option<&str>) -> String {
        match Self::parse_to_local(reset_time_str) {
            Some(dt) => {
                let month = match dt.month() {
                    1 => "jan",
                    2 => "feb",
                    3 => "mar",
                    4 => "apr",
                    5 => "may",
                    6 => "jun",
                    7 => "jul",
                    8 => "aug",
                    9 => "sep",
                    10 => "oct",
                    11 => "nov",
                    12 => "dec",
                    _ => "???",
                };
                format!(
                    "{} {}, {:02}:{:02}",
                    month,
                    dt.day(),
                    dt.hour(),
                    dt.minute()
                )
            }
            None => "?".to_string(),
        }
    }

    pub(crate) fn get_cache_path() -> Option<std::path::PathBuf> {
        let home = dirs::home_dir()?;
        Some(
            home.join(".claude")
                .join("ccline")
                .join(".api_usage_cache.json"),
        )
    }

    /// Shared cache path used by the bash statusline script.
    /// Located at /tmp/claude/statusline-usage-cache.json.
    fn get_shared_cache_path() -> std::path::PathBuf {
        std::path::PathBuf::from("/tmp/claude/statusline-usage-cache.json")
    }

    pub(crate) fn load_cache() -> Option<ApiUsageCache> {
        let cache_path = Self::get_cache_path()?;
        let content = std::fs::read_to_string(&cache_path).ok()?;
        serde_json::from_str(&content).ok()
    }

    /// Try to load usage data from the shared cache at /tmp/claude/.
    /// The shared cache stores the raw API response (ApiUsageResponse format).
    fn load_shared_cache() -> Option<ApiUsageResponse> {
        let path = Self::get_shared_cache_path();
        let content = std::fs::read_to_string(&path).ok()?;
        serde_json::from_str(&content).ok()
    }

    /// Load shared cache only if its mtime is within `max_age_secs`.
    fn load_shared_cache_if_fresh(max_age_secs: u64) -> Option<ApiUsageResponse> {
        let path = Self::get_shared_cache_path();
        let metadata = std::fs::metadata(&path).ok()?;
        let modified = metadata.modified().ok()?;
        let age = std::time::SystemTime::now()
            .duration_since(modified)
            .unwrap_or_default();
        if age.as_secs() >= max_age_secs {
            return None;
        }
        let content = std::fs::read_to_string(&path).ok()?;
        serde_json::from_str(&content).ok()
    }

    /// Save raw API response to the shared cache for other tools to use.
    fn save_shared_cache(response: &ApiUsageResponse) {
        let path = Self::get_shared_cache_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string(response) {
            let _ = std::fs::write(&path, json);
        }
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

        // Get the locally installed version via `claude --version`
        if let Ok(output) = Command::new("claude").arg("--version").output() {
            if output.status.success() {
                let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
                // Output is "X.Y.Z (Claude Code)" — extract just the version
                if let Some(version) = raw.split_whitespace().next() {
                    if !version.is_empty() {
                        return format!("claude-code/{}", version);
                    }
                }
            }
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

impl UsageSegment {
    /// Convert an API response into our internal cache format.
    fn response_to_cache(response: &ApiUsageResponse) -> ApiUsageCache {
        let extra = response.extra_usage.as_ref();
        ApiUsageCache {
            five_hour_utilization: response.five_hour.utilization,
            seven_day_utilization: response.seven_day.utilization,
            resets_at: response.seven_day.resets_at.clone(),
            five_hour_resets_at: response.five_hour.resets_at.clone(),
            cached_at: Utc::now().to_rfc3339(),
            extra_usage_enabled: extra.is_some_and(|e| e.is_enabled),
            extra_usage_utilization: extra.map_or(0.0, |e| e.utilization),
            extra_usage_used_credits: extra.map_or(0.0, |e| e.used_credits),
            extra_usage_monthly_limit: extra.map_or(0.0, |e| e.monthly_limit),
        }
    }
}

impl Segment for UsageSegment {
    fn collect(&self, _input: &InputData) -> Option<SegmentData> {
        // Load config for segment options
        let config = crate::config::Config::load().ok()?;
        let segment_config = config.segments.iter().find(|s| s.id == SegmentId::Usage);

        let cache_duration = segment_config
            .and_then(|sc| sc.options.get("cache_duration"))
            .and_then(|v| v.as_u64())
            .unwrap_or(60);

        // 1. Check ccline cache first
        let cached_data = Self::load_cache();
        if let Some(cache) = cached_data.as_ref() {
            if Self::is_cache_valid(cache, cache_duration) {
                return Self::build_segment_data(
                    cache.five_hour_utilization,
                    cache.five_hour_resets_at.as_deref(),
                );
            }
        }

        // 2. Check shared cache (/tmp/claude/statusline-usage-cache.json) via mtime
        let shared_data = Self::load_shared_cache_if_fresh(cache_duration);
        if let Some(shared) = shared_data.as_ref() {
            let cache = Self::response_to_cache(shared);
            self.save_cache(&cache);
            return Self::build_segment_data(
                shared.five_hour.utilization,
                shared.five_hour.resets_at.as_deref(),
            );
        }

        // 3. Try API fetch (needs token)
        if let Some(token) = credentials::get_oauth_token() {
            let api_base_url = segment_config
                .and_then(|sc| sc.options.get("api_base_url"))
                .and_then(|v| v.as_str())
                .unwrap_or("https://api.anthropic.com");
            let timeout = segment_config
                .and_then(|sc| sc.options.get("timeout"))
                .and_then(|v| v.as_u64())
                .unwrap_or(2);

            if let Some(response) = self.fetch_api_usage(api_base_url, &token, timeout) {
                let cache = Self::response_to_cache(&response);
                self.save_cache(&cache);
                Self::save_shared_cache(&response);
                return Self::build_segment_data(
                    response.five_hour.utilization,
                    response.five_hour.resets_at.as_deref(),
                );
            }
        }

        // 4. Fall back to any stale data: ccline cache, then shared cache (ignoring mtime)
        if let Some(cache) = cached_data {
            return Self::build_segment_data(
                cache.five_hour_utilization,
                cache.five_hour_resets_at.as_deref(),
            );
        }
        if let Some(shared) = Self::load_shared_cache() {
            let cache = Self::response_to_cache(&shared);
            self.save_cache(&cache);
            return Self::build_segment_data(
                shared.five_hour.utilization,
                shared.five_hour.resets_at.as_deref(),
            );
        }

        None
    }

    fn id(&self) -> SegmentId {
        SegmentId::Usage
    }
}

impl UsageSegment {
    fn build_segment_data(
        five_hour_util: f64,
        five_hour_resets_at: Option<&str>,
    ) -> Option<SegmentData> {
        let dynamic_icon =
            super::hourglass_icon_for_utilization(five_hour_util / 100.0).to_string();
        let five_hour_percent = five_hour_util.round() as u8;
        let reset_time = Self::format_time_only(five_hour_resets_at);
        let primary = format!("{}% @{}", five_hour_percent, reset_time);

        let mut metadata = HashMap::new();
        metadata.insert("dynamic_icon".to_string(), dynamic_icon);
        metadata.insert(
            "five_hour_utilization".to_string(),
            five_hour_util.to_string(),
        );

        Some(SegmentData {
            primary,
            secondary: String::new(),
            metadata,
        })
    }
}
