# FFmpeg Studio Distribution

## Installation

### Debian/Ubuntu (.deb)
```bash
sudo dpkg -i "FFmpeg Studio_1.0.0_amd64.deb"
sudo apt-get install -f  # Install dependencies
```

### Fedora/RHEL/CentOS (.rpm)
```bash
sudo rpm -i "FFmpeg Studio-1.0.0-1.x86_64.rpm"
```

### Arch Linux
```bash
# Install from AUR or manually:
makepkg -si
```

### Standalone Binary
```bash
./ffmpeg-studio
```

## Features
- **Convert** - Change format (MP4, AVI, MKV, MOV, WEBM) with re-encoding
- **Remux** - Change container without re-encoding (fast, may not work for all combinations)
- **Compress** - Reduce file size with quality control
- **Resize** - Change resolution
- **Trim** - Cut a portion of the video
- **Extract Audio** - Pull audio track as MP3
- **Create GIF** - Convert video to animated GIF
- **Rotate** - Rotate video 90/180/270 degrees
- **Add Watermark** - Add text watermark to video

## Requirements
- FFmpeg installed on system
- GTK3 (for Linux desktop)
