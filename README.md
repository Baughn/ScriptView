# ScriptView

A real-time subtitle viewer for MPV video player that displays subtitle history in a GUI window.

## Overview

ScriptView is a two-component system that tracks and displays subtitles from MPV:

1. **MPV Lua Script** - Monitors subtitle changes and exports them to a JSON file
2. **Rust GUI Application** - Displays subtitle history in real-time with egui

Perfect for language learning, accessibility, or following along with video content.

## Features

- Real-time subtitle display with bottom alignment (most recent at bottom)
- Always-on-top window option
- Configurable display count (1-50 subtitles)
- One-click script installation for MPV
- Syncplay compatibility
- Automatic subtitle history clearing on file changes and seeks

## Installation

### Prerequisites

- Rust (for building the GUI application)
- MPV video player

### Build

```bash
git clone <repository-url>
cd scriptview
cargo build --release
```

### MPV Script Setup

The Lua script can be installed either:

1. **Via GUI** (recommended): Run the application and click "Install Script" if prompted
2. **Manually**: Copy `subtitle-monitor.lua` to `~/.config/mpv/scripts/`

## Usage

1. Start the ScriptView application:
   ```bash
   cargo run --release
   ```

2. Start MPV with a video file:
   ```bash
   mpv video.mkv
   ```

3. Subtitles will automatically appear in the ScriptView window as they're displayed in MPV

## Configuration

- **Display Count**: Adjust how many recent subtitles to show (1-50)
- **Always on Top**: Toggle window to stay above other applications
- **Script Status**: Monitor installation and runtime status

## How It Works

The system uses file-based inter-process communication:

1. MPV Lua script monitors subtitle changes via the `sub-text` property
2. Script writes subtitle data to `/tmp/mpv-subtitles.json`
3. Rust application watches for file changes and updates the GUI
4. New subtitles appear at the bottom like a chat interface

## License

MIT License - see [LICENSE](LICENSE) file for details.