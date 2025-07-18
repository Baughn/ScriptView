# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

ScriptView is a two-component system for tracking and displaying subtitle history from MPV video player:

1. **MPV Lua Script** (`subtitle-monitor.lua`) - Monitors subtitle changes in MPV and writes them to `/tmp/mpv-subtitles.json`
2. **Rust GUI Application** (`src/main.rs`) - Displays subtitle history in a real-time window with egui

## Build and Run Commands

```bash
# Build the application
cargo build

# Run the application
cargo run

# Build for release
cargo build --release
```

## MPV Integration Setup

The Lua script can be installed either:
- Manually: Copy `subtitle-monitor.lua` to `~/.config/mpv/scripts/`
- Via app: Use the "Install Script" button in the GUI when the script is not detected

Start MPV with the script:
```bash
mpv --script=subtitle-monitor.lua video.mkv
# OR if installed globally:
mpv video.mkv
```

## Architecture

### Inter-Process Communication
- **File-based IPC**: MPV Lua script writes JSON to `/tmp/mpv-subtitles.json`
- **File monitoring**: Rust app uses `notify` crate to watch for file changes
- **JSON format**: Array of `SubtitleEntry` objects with text, timing, and timestamps

### Data Flow
1. MPV displays subtitle → Lua script captures via `sub-text` property
2. Script writes to JSON file → File watcher triggers in Rust app
3. Rust app parses JSON → Updates GUI with new subtitle data
4. GUI displays last N subtitles with bottom alignment (most recent at bottom)

### Key Components

**SubtitleEntry** - Core data structure for subtitle information:
- `text`: Subtitle content
- `start_time`: Video timestamp when subtitle appeared  
- `timestamp`: Unix timestamp when captured
- `end_time`: Optional end time

**SubtitleViewer** - Main GUI application state:
- Manages subtitle history in `Arc<Mutex<Vec<SubtitleEntry>>>`
- Handles file watching and JSON parsing
- Tracks script installation status and file existence separately
- Controls display count and always-on-top behavior

### Lua Script Behavior
- Monitors both primary (`sub-text`) and secondary (`secondary-sub-text`) subtitle tracks
- Automatically clears history on file changes and seeks >5 seconds
- Maintains rolling buffer of last 50 subtitles
- Writes JSON atomically on each subtitle change

### GUI Features
- **Bottom-aligned display**: New subtitles appear at bottom like a chat
- **Always-on-top toggle**: Runtime window level control
- **Smart warnings**: Separate status for script installation vs MPV running
- **Auto-installation**: One-click script deployment to MPV directory
- **Configurable display count**: Show 1-50 most recent subtitles

## Syncplay Compatibility

The system is designed to work with Syncplay-controlled MPV instances. Since Syncplay uses MPV's IPC socket, the Lua script approach avoids socket conflicts by running inside MPV's process.

## File Paths

- Subtitle data: `/tmp/mpv-subtitles.json` 
- Script installation: `~/.config/mpv/scripts/subtitle-monitor.lua`
- Always commit changes after confirming they work (per user global config)

## Version Control
**IMPORTANT**: This project uses Jujutsu (jj) instead of Git. DO NOT use git commands.

```bash
jj status          # Show working copy changes
jj diff            # Show diff of changes
jj commit -m "feat(module): Add feature"  # Commit with Conventional Commits format
jj squash          # Squash into previous commit
jj log --limit 5   # Show recent commits
jj undo            # Undo last operation if mistake made
```

