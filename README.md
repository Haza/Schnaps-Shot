# Schnaps-Shot

**Schnaps-Shot** is a Rust command-line tool that adds stylish borders and EXIF metadata to your JPG and PNG photos.

## ✨ Features
- Add borders to photos
- Extract and overlay EXIF metadata (camera, lens, exposure, etc.)
- Support for JPG and PNG
- Designed for seamless integration as a **Lightroom Export Action**

## 🚀 Installation
```bash
# Build with Cargo
cargo build --release
```

The binary will be available in `target/release/schnapsshot`.

## 📸 Usage
Schnaps-Shot can be used directly from the command line, but its **primary intended usage** is as a Lightroom *Export Action* to automatically process photos after export.

```bash
schnapsshot <input.jpg> --output output.jpg
```

### Options
- `--output <file>`: Path to save the processed image
- `--font <path>`: Custom font for EXIF overlay
- `--help`: Show full list of options

## 🛠 Dependencies
- [clap](https://crates.io/crates/clap)
- [image](https://crates.io/crates/image)
- [imageproc](https://crates.io/crates/imageproc)
- [rusttype](https://crates.io/crates/rusttype)
- [kamadak-exif](https://crates.io/crates/kamadak-exif)
- [palette](https://crates.io/crates/palette)
- [kmeans_colors](https://crates.io/crates/kmeans_colors)

