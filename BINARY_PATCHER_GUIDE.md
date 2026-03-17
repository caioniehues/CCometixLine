# Binary Patcher Testing Guide

This guide walks you through testing the new binary patcher that removes the **token total** and **version display** from Claude Code's native binary.

## What the patcher does

The patcher modifies 4 locations in the Claude Code binary (5 bytes total):

| Patch | Target | Byte change | Effect |
|-------|--------|-------------|--------|
| Native version display | `f&&IX.createElement(...)` | `f` → `0` | Hides `current: X.Y.Z · latest: X.Y.Z` |
| npm version display | `f&&aM.createElement(...)` | `f` → `0` | Hides `globalVersion: X · latestVersion: Y` |
| Package manager version display | `A&&fZ.createElement(...)` | `A` → `0` | Hides `currentVersion: X.Y.Z` |
| Token total display | `!Y&&j9.createElement(...)` | `!Y` → ` 0` | Hides `N tokens` on the right |

All replacements are **same-length** so the binary size never changes.

## Prerequisites

- Rust toolchain (stable)
- A Claude Code native binary installation (the patcher auto-detects ELF and Mach-O formats)

## Step 1: Build the patcher

```bash
cd ~/Code/CCometixLine
cargo build --release
```

The binary will be at `./target/release/ccometixline`.

## Step 2: Locate your Claude Code binary

```bash
# Check where claude is installed
which claude
readlink -f $(which claude)
```

Typical locations:
- **Native install**: `~/.local/share/claude/versions/<version>` (Linux)
- **npm global**: `/usr/lib/node_modules/@anthropic-ai/claude-code/cli.js`
- **npm local**: `~/.claude/local/node_modules/@anthropic-ai/claude-code/cli.js`

## Step 3: Test on a copy first (recommended)

**Do not patch your real binary until you've verified on a copy.**

```bash
# Find your binary
CLAUDE_BIN=$(readlink -f $(which claude))
echo "Claude binary: $CLAUDE_BIN"

# Copy it to a temp location
cp "$CLAUDE_BIN" /tmp/claude-test
```

## Step 4: Run the patcher on the copy

```bash
./target/release/ccometixline --patch /tmp/claude-test
```

### Expected output

```
🔧 Claude Code Patcher
Target file: /tmp/claude-test
📦 Created backup: /tmp/claude-test.backup
Detected native binary (ELF)

🔄 Applying binary patches...
  Found native version display anchor at byte XXXXXXX
  Replacing condition 'f' (byte XXXXXXX) with '0' for native version display
  ...

📊 Binary Patch Results:
  ✅ Native version display
  ✅ npm version display
  ✅ Package manager version display
  ✅ Token total display

✅ All 4 binary patches applied successfully!
```

### What to check

1. **All 4 patches show ✅** — if any show ❌, that patch target wasn't found (may differ between Claude Code versions)
2. **File size unchanged**:
   ```bash
   stat -c '%s' /tmp/claude-test /tmp/claude-test.backup
   # Both should show the exact same number
   ```
3. **Diffs look correct** — each diff should show a single character changing (e.g., `f` → `0`)

## Step 5: Verify the patched binary runs

```bash
/tmp/claude-test --version
```

If this prints the version number without crashing, the binary is intact.

## Step 6: Verify safety checks

### Idempotency (patching twice)

```bash
./target/release/ccometixline --patch /tmp/claude-test
```

Expected:
```
ℹ️ This binary appears to already be patched. Skipping.
```

### Non-Claude binary rejection

```bash
cp /usr/bin/ls /tmp/ls-test
./target/release/ccometixline --patch /tmp/ls-test
```

Expected:
```
❌ This binary does not appear to be Claude Code.
   Expected to find Claude Code markers in the binary.
   Aborting to prevent accidental corruption.
```

## Step 7: Apply to the real binary

Once you're satisfied with the test results:

```bash
CLAUDE_BIN=$(readlink -f $(which claude))
./target/release/ccometixline --patch "$CLAUDE_BIN"
```

This will:
1. Create a backup at `<path>.backup`
2. Validate it's a Claude Code binary
3. Check it hasn't been patched already
4. Apply all 4 patches
5. Save atomically (writes to `.tmp` then renames)

## Step 8: Verify in a live session

Open a new Claude Code session and check:

- [ ] The `N tokens` counter on the right is **gone**
- [ ] The `current: X.Y.Z · latest: X.Y.Z` line on the right is **gone**
- [ ] The CCometixLine statusline at the bottom still works normally
- [ ] Claude Code responds to prompts normally
- [ ] Tool use (file reads, edits, bash) works normally

## Restoring the original binary

If anything goes wrong:

```bash
CLAUDE_BIN=$(readlink -f $(which claude))
cp "${CLAUDE_BIN}.backup" "$CLAUDE_BIN"
```

Or if Claude auto-updated and overwrote your patched version, just re-run the patcher on the new version.

## Notes

- **Claude updates will overwrite the patch.** After each `claude update`, you need to re-run the patcher.
- **The patcher only works on native binaries (ELF/Mach-O).** If you installed via npm with a `cli.js`, the existing text-mode patcher handles that automatically — just point `--patch` at the `cli.js` file.
- **Backup files** are created at `<original-path>.backup`. Only the most recent backup is kept (each run overwrites the previous backup).
