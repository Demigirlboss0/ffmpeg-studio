# FFmpeg Studio

A simple, elegant desktop interface for FFmpeg operations. Built with Tauri + TypeScript.

![FFmpeg Studio](https://github.com/Demigirlboss0/ffmpeg-studio/workflows/build/badge.svg)

## Features

- **Convert** - Change format (MP4, AVI, MKV, MOV, WEBM) with re-encoding
- **Remux** - Change container without re-encoding (fast, may not work for all combinations)
- **Compress** - Reduce file size with quality control (CRF)
- **Resize** - Change video resolution
- **Trim** - Cut a portion of the video
- **Extract Audio** - Pull audio track as MP3
- **Create GIF** - Convert video to animated GIF
- **Rotate** - Rotate video 90/180/270 degrees
- **Add Watermark** - Add text watermark to video

## Theme

Desert Rose:
- Dusty Rose (#d4a5a5)
- Clay (#b87d6d)
- Sand (#e8d5c4)
- Deep Burgundy (#5d2e46)

## Installation

### Debian/Ubuntu
```bash
sudo dpkg -i FFmpeg_Studio_1.0.0_amd64.deb
sudo apt-get install -f
```

### Fedora/RHEL/CentOS
```bash
sudo rpm -i FFmpeg_Studio-1.0.0-1.x86_64.rpm
```

### Arch Linux
```bash
# From AUR or manually:
makepkg -si
```

### Standalone Binary
```bash
./ffmpeg-studio
```

## Requirements

- FFmpeg installed on system
- GTK3 (for Linux desktop)

## Development

```bash
cd frontend
npm install
npm run tauri dev
```

## Build

```bash
cd frontend
npm run tauri build
```

## License

MIT
