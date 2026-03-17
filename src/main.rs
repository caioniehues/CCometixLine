use ccometixline::cli::Cli;
use ccometixline::config::{Config, InputData};
use ccometixline::core::{collect_all_segments, StatusLineGenerator};
use ccometixline::ui::{MainMenu, MenuResult};
use std::io::{self, IsTerminal};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse_args();

    if cli.config {
        ccometixline::ui::run_configurator()?;
        return Ok(());
    }

    // Handle Claude Code patcher
    if let Some(claude_path) = cli.patch {
        println!("🔧 Claude Code Patcher");
        println!("Target file: {}", claude_path);

        // Create backup in same directory
        let backup_path = format!("{}.backup", claude_path);
        std::fs::copy(&claude_path, &backup_path)?;
        println!("📦 Created backup: {}", backup_path);

        // Detect file type by reading first 4 bytes
        let magic = {
            use std::io::Read;
            let mut f = std::fs::File::open(&claude_path)?;
            let mut buf = [0u8; 4];
            f.read_exact(&mut buf)?;
            buf
        };

        let is_elf = &magic == b"\x7FELF";
        let is_macho = &magic == b"\xCF\xFA\xED\xFE"
            || &magic == b"\xFE\xED\xFA\xCF"
            || &magic == b"\xCA\xFE\xBA\xBE";

        if is_elf || is_macho {
            use ccometixline::utils::BinaryPatcher;
            let kind = if is_elf { "ELF" } else { "Mach-O" };
            println!("Detected native binary ({})", kind);

            let mut patcher = BinaryPatcher::new(&claude_path)?;

            if !patcher.validate_claude_binary() {
                println!("❌ This binary does not appear to be Claude Code.");
                println!("   Expected to find Claude Code markers in the binary.");
                println!("   Aborting to prevent accidental corruption.");
                return Ok(());
            }

            if patcher.is_already_patched() {
                println!("ℹ️ This binary appears to already be patched. Skipping.");
                return Ok(());
            }

            println!("\n🔄 Applying binary patches...");
            let results = patcher.apply_all_patches();
            patcher.save()?;
            BinaryPatcher::print_summary(&results);
        } else {
            use ccometixline::utils::ClaudeCodePatcher;
            println!("Detected text file");
            println!("\n🔄 Applying patches...");
            let mut patcher = ClaudeCodePatcher::new(&claude_path)?;
            let results = patcher.apply_all_patches();
            patcher.save()?;
            ClaudeCodePatcher::print_summary(&results);
        }

        println!("💡 To restore, replace with the backup file:");
        println!("   cp {} {}", backup_path, claude_path);

        return Ok(());
    }

    // Load configuration
    let mut config = Config::load().unwrap_or_else(|_| Config::default());

    // Apply theme override if provided
    if let Some(theme) = cli.theme {
        config = ccometixline::ui::themes::ThemePresets::get_theme(&theme);
    }

    // Check if stdin has data
    if io::stdin().is_terminal() {
        if let Some(result) = MainMenu::run()? {
            match result {
                MenuResult::LaunchConfigurator => {
                    ccometixline::ui::run_configurator()?;
                }
                MenuResult::InitConfig | MenuResult::CheckConfig => {}
                MenuResult::Exit => {}
            }
        }
        return Ok(());
    }

    // Read Claude Code data from stdin
    let stdin = io::stdin();
    let input: InputData = serde_json::from_reader(stdin.lock())?;

    // Collect segment data
    let segments_data = collect_all_segments(&config, &input);

    // Render statusline
    let generator = StatusLineGenerator::new(config);
    let statusline = generator.generate(segments_data);

    println!("{}", statusline);

    Ok(())
}
