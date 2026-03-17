use memchr::memmem;
use std::fs;
use std::path::Path;

/// A patch to apply: byte range + same-length replacement
#[derive(Debug)]
struct BinaryPatch {
    start: usize,
    end: usize,
    replacement: Vec<u8>,
    label: String,
}

/// Patches native Claude Code binaries (Bun single-executable ELF files).
///
/// Unlike `ClaudeCodePatcher` which parses JavaScript AST via tree-sitter,
/// this operates on raw bytes with same-length replacements to avoid
/// corrupting the binary's ELF structure.
///
/// Bun bundles embed the JS source **twice** in the binary, so all patches
/// must be applied to every occurrence.
#[derive(Debug)]
pub struct BinaryPatcher {
    file_content: Vec<u8>,
    file_path: String,
}

impl BinaryPatcher {
    pub fn new<P: AsRef<Path>>(file_path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let path = file_path.as_ref();
        let content = fs::read(path)?;

        Ok(Self {
            file_content: content,
            file_path: path.to_string_lossy().to_string(),
        })
    }

    /// Save the patched binary using atomic write (temp file + rename)
    /// to prevent corruption if the process is killed mid-write.
    /// Preserves the original file's permissions (importantly, the execute bit).
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let temp_path = format!("{}.tmp", self.file_path);

        // Capture original permissions before overwriting
        let original_perms = fs::metadata(&self.file_path)?.permissions();

        fs::write(&temp_path, &self.file_content)?;
        fs::set_permissions(&temp_path, original_perms)?;
        fs::rename(&temp_path, &self.file_path)?;
        Ok(())
    }

    /// Verify the binary is actually a Claude Code / Bun executable
    /// by checking for known embedded strings.
    pub fn validate_claude_binary(&self) -> bool {
        let markers = [b"@anthropic-ai/claude-code" as &[u8], b"claude.ai" as &[u8]];
        markers
            .iter()
            .all(|marker| !self.find_all_bytes(marker).is_empty())
    }

    /// Check if the binary has already been patched (idempotency check).
    /// Looks for our patch signatures — ALL occurrences of a known anchor
    /// must have their condition byte set to '0'.
    pub fn is_already_patched(&self) -> bool {
        let anchor = b"\"current: \",_.current";
        let positions = self.find_all_bytes(anchor);
        if positions.is_empty() {
            return false;
        }
        // All occurrences must be patched
        positions.iter().all(|&anchor_pos| {
            let create_elem = b".createElement(";
            if let Some(ce_pos) = self.find_bytes_backwards(create_elem, anchor_pos + 1, 300) {
                if let Some(aa_pos) = self.find_bytes_backwards(b"&&", ce_pos, 100) {
                    if let Some(cond_pos) = aa_pos.checked_sub(1) {
                        return self.file_content[cond_pos] == b'0';
                    }
                }
            }
            false
        })
    }

    // =========================================================================
    // Byte search helpers
    // =========================================================================

    /// Find ALL occurrences of `needle` in the entire file content.
    fn find_all_bytes(&self, needle: &[u8]) -> Vec<usize> {
        let mut positions = Vec::new();
        let finder = memmem::Finder::new(needle);
        let mut offset = 0;
        while offset < self.file_content.len() {
            if let Some(pos) = finder.find(&self.file_content[offset..]) {
                positions.push(offset + pos);
                offset += pos + 1; // move past this match
            } else {
                break;
            }
        }
        positions
    }

    /// Find `needle` searching backwards from `from` within a window.
    fn find_bytes_backwards(&self, needle: &[u8], from: usize, window: usize) -> Option<usize> {
        let start = from.saturating_sub(window);
        let end = from.min(self.file_content.len());
        if start >= end || needle.is_empty() || needle.len() > (end - start) {
            return None;
        }
        let slice = &self.file_content[start..end];
        for i in (0..=slice.len() - needle.len()).rev() {
            if &slice[i..i + needle.len()] == needle {
                return Some(start + i);
            }
        }
        None
    }

    /// Apply a same-length replacement. Panics if lengths differ.
    fn safe_replace(&mut self, start: usize, end: usize, replacement: &[u8]) {
        let len = end - start;
        assert_eq!(
            replacement.len(),
            len,
            "Binary patch must be same byte length: expected {} bytes, got {}",
            len,
            replacement.len()
        );
        self.file_content[start..end].copy_from_slice(replacement);
    }

    // =========================================================================
    // Patch finders — return patches for ALL occurrences
    // =========================================================================

    /// Find and disable ALL occurrences of the native auto-updater version display.
    /// Targets: `f&&IX.createElement(E,{dimColor:!0,wrap:"truncate"},"current: ",_.current,...)`
    fn find_version_display_patches(&self, anchor: &[u8], name: &'static str) -> Vec<BinaryPatch> {
        let positions = self.find_all_bytes(anchor);
        let mut patches = Vec::new();

        for (idx, &anchor_pos) in positions.iter().enumerate() {
            let label = if positions.len() > 1 {
                format!("{} (occurrence {})", name, idx + 1)
            } else {
                name.to_string()
            };

            if let Some(mut patch) = self.find_condition_before_anchor(anchor_pos, &label) {
                patch.label = label;
                patches.push(patch);
            } else {
                println!(
                    "  ⚠️ Could not find condition for {} at byte {}",
                    name, anchor_pos
                );
            }
        }

        patches
    }

    /// Find and disable ALL occurrences of the token total display.
    ///
    /// The token display structure is:
    /// `...vH?[createElement(m,{key:"tokens"}, ..., createElement(E,{dimColor:!0},JH," tokens"))]:[]`
    ///
    /// `vH` is the outer ternary condition. We need to target it (not the inner `!Y`).
    /// The anchor is `key:"tokens"` and the ternary `?` is before the `[`.
    fn find_token_display_patches(&self) -> Vec<BinaryPatch> {
        let anchor = b"key:\"tokens\"";
        let positions = self.find_all_bytes(anchor);
        let mut patches = Vec::new();

        for (idx, &anchor_pos) in positions.iter().enumerate() {
            let label = if positions.len() > 1 {
                format!("Token total display (occurrence {})", idx + 1)
            } else {
                "Token total display".to_string()
            };

            // The structure is: ...vH?[createElement(m,{...key:"tokens"...},...," tokens"))]:[]
            // We need to find the `?[` before the createElement, then the condition before `?`
            // Search backwards from anchor for `?[` which starts the ternary true branch
            if let Some(patch) = self.find_ternary_condition(anchor_pos, &label) {
                patches.push(patch);
            } else {
                println!(
                    "  ⚠️ Could not find ternary condition for token display at byte {}",
                    anchor_pos
                );
            }
        }

        patches
    }

    /// Find the condition of a ternary `condition?[...]:[]` given a position inside the `[...]`.
    /// Replaces the condition variable with `0` so the ternary always takes the `[]` branch.
    fn find_ternary_condition(&self, inside_pos: usize, label: &str) -> Option<BinaryPatch> {
        // Search backwards for `?[` which starts the ternary's true branch
        let question_bracket = b"?[";
        let qb_pos = self.find_bytes_backwards(question_bracket, inside_pos, 200)?;

        // The condition is immediately before `?`
        let condition_byte_pos = qb_pos.checked_sub(1)?;
        let original_byte = self.file_content[condition_byte_pos];

        // Validate: should be an identifier character
        if !original_byte.is_ascii_alphanumeric() && original_byte != b'_' && original_byte != b'$'
        {
            println!(
                "  ⚠️ Unexpected byte '{}' (0x{:02x}) before ?[ at {} for {}",
                original_byte as char, original_byte, qb_pos, label
            );
            return None;
        }

        // Check for negation
        let has_negation =
            condition_byte_pos > 0 && self.file_content[condition_byte_pos - 1] == b'!';

        if has_negation {
            let start = condition_byte_pos - 1;
            println!(
                "  Replacing ternary condition '!{}' (bytes {}-{}) with ' 0' for {}",
                original_byte as char, start, condition_byte_pos, label
            );
            Some(BinaryPatch {
                start,
                end: condition_byte_pos + 1,
                replacement: vec![b' ', b'0'],
                label: label.to_string(),
            })
        } else {
            println!(
                "  Replacing ternary condition '{}' (byte {}) with '0' for {}",
                original_byte as char, condition_byte_pos, label
            );
            Some(BinaryPatch {
                start: condition_byte_pos,
                end: condition_byte_pos + 1,
                replacement: vec![b'0'],
                label: label.to_string(),
            })
        }
    }

    // =========================================================================
    // Common: find and neutralize a `&&` condition before an anchor
    // =========================================================================

    /// Given a position near a `createElement` call, search backwards for the
    /// `&&` condition that gates it, and replace the condition byte with `0`.
    fn find_condition_before_anchor(
        &self,
        anchor_pos: usize,
        patch_name: &str,
    ) -> Option<BinaryPatch> {
        let create_elem = b".createElement(";
        let ce_pos = self.find_bytes_backwards(create_elem, anchor_pos + 1, 300)?;

        let and_and = b"&&";
        let aa_pos = self.find_bytes_backwards(and_and, ce_pos, 100)?;

        let condition_byte_pos = aa_pos.checked_sub(1)?;
        let original_byte = self.file_content[condition_byte_pos];

        if !original_byte.is_ascii_alphanumeric()
            && original_byte != b'_'
            && original_byte != b'$'
            && original_byte != b')'
        {
            println!(
                "  ⚠️ Unexpected condition byte '{}' (0x{:02x}) before && at {} for {}",
                original_byte as char, original_byte, aa_pos, patch_name
            );
            return None;
        }

        if original_byte == b')' {
            return self.find_condition_expr_before_and(aa_pos, patch_name);
        }

        let has_negation =
            condition_byte_pos > 0 && self.file_content[condition_byte_pos - 1] == b'!';

        if has_negation {
            let start = condition_byte_pos - 1;
            println!(
                "  Replacing negated condition '!{}' (bytes {}-{}) with ' 0' for {}",
                original_byte as char, start, condition_byte_pos, patch_name
            );
            Some(BinaryPatch {
                start,
                end: condition_byte_pos + 1,
                replacement: vec![b' ', b'0'],
                label: patch_name.to_string(),
            })
        } else {
            println!(
                "  Replacing condition '{}' (byte {}) with '0' for {}",
                original_byte as char, condition_byte_pos, patch_name
            );
            Some(BinaryPatch {
                start: condition_byte_pos,
                end: condition_byte_pos + 1,
                replacement: vec![b'0'],
                label: patch_name.to_string(),
            })
        }
    }

    /// Handle conditions like `!someFunc()&&` where the byte before `&&` is `)`.
    fn find_condition_expr_before_and(
        &self,
        and_pos: usize,
        patch_name: &str,
    ) -> Option<BinaryPatch> {
        let close_paren_pos = and_pos - 1;

        let mut depth = 1;
        let mut pos = close_paren_pos;
        while depth > 0 && pos > 0 {
            pos -= 1;
            match self.file_content[pos] {
                b')' => depth += 1,
                b'(' => depth -= 1,
                _ => {}
            }
        }

        if depth != 0 {
            return None;
        }

        let mut start = pos;
        while start > 0 {
            let b = self.file_content[start - 1];
            if b.is_ascii_alphanumeric() || b == b'_' || b == b'$' || b == b'.' || b == b'!' {
                start -= 1;
            } else {
                break;
            }
        }

        let expr_len = close_paren_pos + 1 - start;
        let original =
            String::from_utf8_lossy(&self.file_content[start..close_paren_pos + 1]).to_string();

        println!(
            "  Replacing expression '{}' ({} bytes at {}) with padded '0' for {}",
            original, expr_len, start, patch_name
        );

        let mut replacement = vec![b' '; expr_len];
        replacement[0] = b'0';

        Some(BinaryPatch {
            start,
            end: close_paren_pos + 1,
            replacement,
            label: patch_name.to_string(),
        })
    }

    // =========================================================================
    // Show diff for debugging
    // =========================================================================

    fn show_diff(&self, patch: &BinaryPatch) {
        let ctx_start = patch.start.saturating_sub(30);
        let ctx_end = (patch.end + 30).min(self.file_content.len());

        let before = String::from_utf8_lossy(&self.file_content[ctx_start..patch.start]);
        let old = String::from_utf8_lossy(&self.file_content[patch.start..patch.end]);
        let after = String::from_utf8_lossy(&self.file_content[patch.end..ctx_end]);
        let new_text = String::from_utf8_lossy(&patch.replacement);

        println!("\n--- {} Diff ---", patch.label);
        println!("OLD: {}\x1b[31m{}\x1b[0m{}", before, old, after);
        println!("NEW: {}\x1b[32m{}\x1b[0m{}", before, new_text, after);
        println!("--- End Diff ---\n");
    }

    // =========================================================================
    // Batch patching
    // =========================================================================

    pub fn apply_all_patches(&mut self) -> Vec<(&'static str, bool)> {
        let mut results = Vec::new();
        let mut all_patches: Vec<BinaryPatch> = Vec::new();

        // 1. Native auto-updater version display
        let native_patches =
            self.find_version_display_patches(b"\"current: \",_.current", "Native version display");
        let found = !native_patches.is_empty();
        for p in native_patches {
            self.show_diff(&p);
            all_patches.push(p);
        }
        if !found {
            println!("⚠️ Could not find native version display");
        }
        results.push(("Native version display", found));

        // 2. npm auto-updater version display
        let npm_patches = self
            .find_version_display_patches(b"\"globalVersion: \",_.global", "npm version display");
        let found = !npm_patches.is_empty();
        for p in npm_patches {
            self.show_diff(&p);
            all_patches.push(p);
        }
        if !found {
            println!("⚠️ Could not find npm version display");
        }
        results.push(("npm version display", found));

        // 3. Package manager auto-updater version display
        let pkg_patches = self.find_version_display_patches(
            b"\"currentVersion: \",{",
            "Package manager version display",
        );
        let found = !pkg_patches.is_empty();
        for p in pkg_patches {
            self.show_diff(&p);
            all_patches.push(p);
        }
        if !found {
            println!("⚠️ Could not find package manager version display");
        }
        results.push(("Package manager version display", found));

        // 4. Token total display — uses ternary condition, not &&
        let token_patches = self.find_token_display_patches();
        let found = !token_patches.is_empty();
        for p in token_patches {
            self.show_diff(&p);
            all_patches.push(p);
        }
        if !found {
            println!("⚠️ Could not find token total display");
        }
        results.push(("Token total display", found));

        // Validate no patches overlap
        for i in 0..all_patches.len() {
            for j in (i + 1)..all_patches.len() {
                let a = &all_patches[i];
                let b = &all_patches[j];
                if a.start < b.end && b.start < a.end {
                    println!(
                        "⚠️ Patches '{}' and '{}' overlap! Skipping all patches.",
                        a.label, b.label
                    );
                    return results;
                }
            }
        }

        // Apply all patches
        for patch in &all_patches {
            println!("Applying: {}", patch.label);
            self.safe_replace(patch.start, patch.end, &patch.replacement);
        }

        println!("Applied {} total byte patches", all_patches.len());

        results
    }

    pub fn print_summary(results: &[(&str, bool)]) {
        println!("\n📊 Binary Patch Results:");
        for (name, success) in results {
            if *success {
                println!("  ✅ {}", name);
            } else {
                println!("  ❌ {}", name);
            }
        }

        let success_count = results.iter().filter(|(_, s)| *s).count();
        let total_count = results.len();

        if success_count == total_count {
            println!(
                "\n✅ All {} binary patches applied successfully!",
                total_count
            );
        } else {
            println!(
                "\n⚠️ {}/{} binary patches applied successfully",
                success_count, total_count
            );
        }
    }
}
