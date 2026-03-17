/// Read and parse ~/.claude/settings.json (respects CLAUDE_CONFIG_DIR).
pub fn load_settings() -> Option<serde_json::Value> {
    let config_dir = std::env::var("CLAUDE_CONFIG_DIR").ok().or_else(|| {
        std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .ok()
            .map(|h| format!("{}/.claude", h))
    })?;
    let path = format!("{}/settings.json", config_dir);
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}
