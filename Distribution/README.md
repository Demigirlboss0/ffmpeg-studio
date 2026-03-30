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

## Requirements
- FFmpeg installed on system
- GTK3 (for Linux desktop)

## Notes
- AppImage bundling is not supported in this release
