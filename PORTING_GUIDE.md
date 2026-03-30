# Converting Python WebUIs to Portable Tauri Desktop Apps

This guide outlines the procedure for converting Python-based web interfaces to portable Linux desktop applications using Tauri + TypeScript.

## Overview

- **Frontend**: TypeScript + Vite (replaces the Python web server)
- **Backend**: Rust with Tauri (replaces Python/Flask)
- **Build Output**: Single portable binary (~7MB) + .deb/.rpm packages

## Prerequisites

```bash
# Install Node.js and npm
curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash -
sudo apt-get install -y nodejs

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Install FFmpeg (required at runtime)
sudo apt install ffmpeg
```

## Step 1: Set Up the Project Structure

Create a new Tauri project:

```bash
mkdir my-app
cd my-app
npm create tauri-app@latest
# Follow prompts:
# - Use TypeScript template
# - App name: my-app
# - Identifier: com.myapp.app
```

Clean up the template files and set up the structure:

```bash
cd my-app
rm -rf src/* src-tauri/src/*
```

## Step 2: Port the Frontend

### Convert HTML/JS to TypeScript

1. Copy your HTML to `index.html`
2. Create `src/main.ts` for JavaScript/TypeScript logic
3. Copy CSS to `src/style.css`
4. Add CSS link to `index.html`:

```html
<link rel="stylesheet" href="./src/style.css">
```

### Key Changes

- Replace `fetch()` calls with Tauri IPC:

```typescript
// Before (Python backend)
const result = await fetch('/process', { ... });

// After (Tauri backend)
import { invoke } from '@tauri-apps/api/core';
const result = await invoke('my_command', { args });
```

- Replace file dialogs:

```typescript
// Before
<input type="file">

// After
import { open, save } from '@tauri-apps/plugin-dialog';
const file = await open({ ... });
```

## Step 3: Port the Backend to Rust

### Define Commands in `src-tauri/src/main.rs`

```rust
use serde::{Deserialize, Serialize};
use tauri::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyRequest {
    pub param1: String,
    pub param2: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyResponse {
    pub success: bool,
    pub result: Option<String>,
    pub error: Option<String>,
}

#[tauri::command]
async fn my_command(request: MyRequest) -> Result<MyResponse, String> {
    // Your logic here
    Ok(MyResponse {
        success: true,
        result: Some("done".to_string()),
        error: None,
    })
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![my_command])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### Key Patterns

- Use `std::process::Command` to run system tools (like FFmpeg)
- Use `tokio::spawn` for async operations
- Return `Result<T, String>` for error handling
- Emit events for progress updates:

```rust
use tauri::{AppHandle, Emitter};

app_clone.emit("progress", ProgressEvent { percent: 50 });
```

## Step 4: Add Logging

```rust
use log::{info, error};

fn main() {
    // Add logging plugin
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::new()
            .level(log::LevelFilter::Debug)
            .build())
        . ...
}

// In commands
fn my_command(...) -> ... {
    info!("Processing request: {:?}", request);
    error!("Failed: {}", err);
    
    // Also print to stderr for immediate feedback
    eprintln!("[INFO] Starting process...");
}
```

## Step 5: Configure Tauri

Edit `src-tauri/tauri.conf.json`:

```json
{
  "productName": "My App",
  "version": "1.0.0",
  "identifier": "com.myapp.app",
  "build": {
    "frontendDist": "../dist",
    "beforeBuildCommand": "npm run build"
  },
  "app": {
    "windows": [{
      "title": "My App",
      "width": 900,
      "height": 700,
      "center": true
    }]
  },
  "bundle": {
    "active": true,
    "targets": ["deb", "rpm"]
  }
}
```

Add capabilities in `src-tauri/capabilities/default.json`:

```json
{
  "identifier": "default",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "dialog:default",
    "dialog:allow-open",
    "dialog:allow-save",
    "fs:default",
    "shell:default"
  ]
}
```

## Step 6: Install Frontend Dependencies

```bash
npm install @tauri-apps/api @tauri-apps/plugin-dialog @tauri-apps/plugin-fs @tauri-apps/plugin-shell
```

## Step 7: Build

```bash
cd my-app
npm run tauri build
```

Output will be in `src-tauri/target/release/`.

## Step 8: Create Distribution Package

Create a `Distribution` folder with:

- `my-app` - Standalone binary
- `my-app_1.0.0_amd64.deb` - Debian package
- `my-app-1.0.0-1.x86_64.rpm` - RPM package
- `PKGBUILD` - For Arch Linux
- `README.md` - Installation instructions
- `LICENSE` - MIT license

## Common Issues & Solutions

### "Connection refused" error
- Ensure frontend is built before running `cargo build`
- Run `npm run build` before `npm run tauri build`
- Check CSP settings in tauri.conf.json

### CSS not loading
- Ensure `<link rel="stylesheet">` is in index.html
- Verify build outputs CSS file in dist/assets/

### Dialog plugin error
- Remove empty `"dialog": {}` from plugins in tauri.conf.json

### Tests failing
- Use `cmd.join(" ")` for string assertions on command vectors

## Key Differences from Python

| Python | Tauri |
|--------|-------|
| `def` | `fn` |
| `requests` | `invoke()` |
| `flask` | Tauri commands |
| `print()` | `eprintln!()` + log crate |
| `subprocess` | `std::process::Command` |
| `async def` | `async fn` + tokio |

## Example: FFmpeg Command Pattern

```rust
fn build_ffmpeg_command(input: &str, output: &str, operation: &str) -> Vec<String> {
    let mut cmd = vec![
        "ffmpeg".to_string(),
        "-y".to_string(),
        "-i".to_string(),
        input.to_string(),
    ];
    
    match operation {
        "convert" => {
            cmd.extend(["-c:v".to_string(), "libx264".to_string()]);
        }
        "remux" => {
            cmd.extend(["-c".to_string(), "copy".to_string()]);
        }
        _ => {}
    }
    
    cmd.push(output.to_string());
    cmd
}
```

## Notes

- Keep FFmpeg as a system dependency (assume user has it installed)
- No need to bundle Python - system Python is used
- Binary is fully portable - just needs GTK3 and FFmpeg
- Test thoroughly on a clean system before distribution
