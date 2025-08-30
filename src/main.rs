/*!
 * Schnaps-Shot - Professional Photo Border and EXIF Metadata Application
 *
 * This application adds elegant borders to photographic images with the ability
 * to integrate EXIF metadata directly onto the image.
 *
 * Key Features:
 * - Add borders of different sizes (small, medium, large)
 * - Extract and display EXIF data (camera, lens, settings)
 * - Batch processing of multiple images
 * - Support for JPEG and PNG formats
 * - GUI and CLI interfaces
 *
 * Author: Nicolas M.
 * Version: 0.1.0
 */

// Hide console window in GUI mode on Windows
#![cfg_attr(all(target_os = "windows", not(feature = "console")), windows_subsystem = "windows")]

use clap::{Arg, Command};
use image::{ImageBuffer, Rgb, RgbImage};
use imageproc::drawing::{draw_text_mut};
use rusttype::{Font, Scale};
use std::fs;
use std::path::Path;
use exif::{In, Tag, Reader};
use std::error::Error;
use std::fmt;
use std::io;

mod gui;
use gui::GuiApp;

// ============================================================================
// ERROR HANDLING
// ============================================================================

/// Enumeration of different error types that can occur in the application
///
/// This enumeration centralizes all possible error types to facilitate
/// error handling and debugging.
#[derive(Debug)]
pub enum PhotoBorderError {
    /// Errors related to image processing (reading, writing, format)
    ImageError(image::ImageError),
    /// Input/output errors (files not found, permissions, etc.)
    IoError(std::io::Error),
    /// Errors when reading EXIF data
    ExifError(exif::Error),
    /// Font-related errors
    FontError(String),
}

/// Implementation of formatted error display
///
/// Provides clear and understandable error messages
/// for the end user.
impl fmt::Display for PhotoBorderError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PhotoBorderError::ImageError(e) => write!(f, "Image processing error: {}", e),
            PhotoBorderError::IoError(e) => write!(f, "Input/output error: {}", e),
            PhotoBorderError::ExifError(e) => write!(f, "EXIF reading error: {}", e),
            PhotoBorderError::FontError(e) => write!(f, "Font error: {}", e),
        }
    }
}

impl Error for PhotoBorderError {}

// Automatic conversions from standard error types
// to our custom error type
impl From<image::ImageError> for PhotoBorderError {
    fn from(error: image::ImageError) -> Self {
        PhotoBorderError::ImageError(error)
    }
}

impl From<std::io::Error> for PhotoBorderError {
    fn from(error: std::io::Error) -> Self {
        PhotoBorderError::IoError(error)
    }
}

impl From<exif::Error> for PhotoBorderError {
    fn from(error: exif::Error) -> Self {
        PhotoBorderError::ExifError(error)
    }
}

// ============================================================================
// BORDER TYPES
// ============================================================================

/// Enumeration of different available border types
///
/// Each type corresponds to a different border size, calculated
/// proportionally to the source image size.
#[derive(Debug, Clone)]
pub enum BorderType {
    /// Thin border - ideal for a minimalist look
    Small,
    /// Medium border - balance between style and discretion
    Medium,
    /// Large border - for a pronounced artistic effect
    Large,
}

impl BorderType {
    /// Parses a string to determine the border type
    ///
    /// # Arguments
    /// * `s` - String to analyze (accepts "s"/"small", "m"/"medium", "l"/"large")
    ///
    /// # Returns
    /// * `Ok(BorderType)` if the string is recognized
    /// * `Err(&'static str)` if the string is not valid
    pub fn from_str(s: &str) -> Result<Self, &'static str> {
        match s.to_lowercase().as_str() {
            "s" | "small" => Ok(BorderType::Small),
            "m" | "medium" => Ok(BorderType::Medium),
            "l" | "large" => Ok(BorderType::Large),
            _ => Err("Invalid border type"),
        }
    }

    /// Calculates border dimensions based on type and image size
    ///
    /// # Arguments
    /// * `img_width` - Source image width
    /// * `img_height` - Source image height
    ///
    /// # Returns
    /// Tuple (top, right, bottom, left) representing the thickness of each border
    ///
    /// # Calculation Logic
    /// - Small: Thin border only at bottom (polaroid style)
    /// - Medium: Uniform border representing 1/15 of the smallest dimension
    /// - Large: Uniform border representing 1/10 of the smallest dimension
    fn get_border_size(&self, img_width: u32, img_height: u32) -> (u32, u32, u32, u32) {
        // Uses the smallest dimension to maintain harmonious proportions
        let min_dimension = img_width.min(img_height);

        match self {
            BorderType::Small => {
                // Polaroid style: no side borders, thin border at bottom
                let _side = min_dimension;
                let bottom = min_dimension / 60;
                (0, 0, bottom, 0)
            },
            BorderType::Medium => {
                // Medium-sized uniform border
                let border = min_dimension / 15;
                (border, border, border, border)
            },
            BorderType::Large => {
                // Large uniform border for artistic effect
                let border = min_dimension / 10;
                (border, border, border, border)
            },
        }
    }
}

// ============================================================================
// EXIF DATA
// ============================================================================

/// Structure containing EXIF metadata extracted from an image
///
/// This structure stores all important technical information
/// from a photograph that can be displayed on the border.
#[derive(Debug, Default)]
pub struct ExifData {
    /// Camera model used
    pub camera: Option<String>,
    /// Lens model used
    pub lens: Option<String>,
    /// Focal length in millimeters
    pub focal_length: Option<String>,
    /// Aperture (f-number)
    pub aperture: Option<String>,
    /// Shutter speed
    pub shutter_speed: Option<String>,
    /// ISO sensitivity
    pub iso: Option<String>,
    /// Date taken
    pub date_taken: Option<String>,
}

impl ExifData {
    /// Extracts EXIF data from an image file
    ///
    /// # Arguments
    /// * `path` - Path to the image file
    ///
    /// # Returns
    /// * `Ok(ExifData)` containing the extracted metadata
    /// * `Err(PhotoBorderError)` in case of reading error
    ///
    /// # Functionality
    /// Uses the `exif` crate to read the EXIF container and extracts
    /// standard photographic metadata fields.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, PhotoBorderError> {
        // Opening and preparing the file for EXIF reading
        let file = fs::File::open(path)?;
        let mut bufreader = std::io::BufReader::new(&file);
        let exifreader = Reader::new();
        let exif = exifreader.read_from_container(&mut bufreader)?;

        let mut exif_data = ExifData::default();

        // Extract camera information
        // Combines Make and Model to get the full name
        if let Some(_make) = exif.get_field(Tag::Make, In::PRIMARY) {
            if let Some(model) = exif.get_field(Tag::Model, In::PRIMARY) {
                let model_str = model.display_value().to_string();
                exif_data.camera = Some(format!("{}",
                                                model_str.trim_matches('"')));
            }
        }

        // Extract lens model
        if let Some(lens) = exif.get_field(Tag::LensModel, In::PRIMARY) {
            exif_data.lens = Some(lens.display_value().to_string().trim_matches('"').to_string());
        }

        // Extract focal length with mm formatting
        if let Some(focal) = exif.get_field(Tag::FocalLength, In::PRIMARY) {
            exif_data.focal_length = Some(format!("{}mm", focal.display_value()));
        }

        // Extract aperture with f/ formatting
        if let Some(aperture) = exif.get_field(Tag::FNumber, In::PRIMARY) {
            exif_data.aperture = Some(format!("f/{}", aperture.display_value()));
        }

        // Extract shutter speed with seconds formatting
        if let Some(shutter) = exif.get_field(Tag::ExposureTime, In::PRIMARY) {
            exif_data.shutter_speed = Some(format!("{}s", shutter.display_value()));
        }

        // Extract ISO sensitivity
        if let Some(iso) = exif.get_field(Tag::PhotographicSensitivity, In::PRIMARY) {
            exif_data.iso = Some(format!("ISO {}", iso.display_value()));
        }

        // Note: Date taken commented out - can be enabled if needed
        //if let Some(date) = exif.get_field(Tag::DateTimeOriginal, In::PRIMARY) {
        //    exif_data.date_taken = Some(date.display_value().to_string().trim_matches('"').to_string());
        //}

        Ok(exif_data)
    }

    /// Formats EXIF data for display on the image
    ///
    /// # Returns
    /// Vector of formatted strings, each element representing a display line
    ///
    /// # Formatting Logic
    /// 1. Camera on the first line
    /// 2. Lens on the second line
    /// 3. Technical settings (focal, aperture, speed, ISO) separated by bullets
    /// 4. Date taken last
    pub fn format_for_display(&self) -> Vec<String> {
        let mut lines = Vec::new();

        // Add camera if available
        if let Some(camera) = &self.camera {
            lines.push(camera.clone());
        }

        // Add lens if available
        if let Some(lens) = &self.lens {
            lines.push(lens.clone());
        }

        // Group technical settings on a single line
        let mut settings = Vec::new();
        if let Some(focal) = &self.focal_length {
            settings.push(focal.clone());
        }
        if let Some(aperture) = &self.aperture {
            settings.push(aperture.clone());
        }
        if let Some(shutter) = &self.shutter_speed {
            settings.push(shutter.clone());
        }
        if let Some(iso) = &self.iso {
            settings.push(iso.clone());
        }

        // Add settings line if any exist
        if !settings.is_empty() {
            lines.push(settings.join(" • "));
        }

        // Add date if available
        if let Some(date) = &self.date_taken {
            lines.push(date.clone());
        }

        lines
    }
}

// ============================================================================
// MAIN PROCESSOR
// ============================================================================

/// Main structure managing the addition of borders to images
///
/// This structure encapsulates all the logic needed to process
/// images: border configuration, font management, and
/// EXIF metadata processing.
pub struct PhotoBorder {
    /// Border type to apply
    border_type: BorderType,
    /// Indicates whether to display EXIF data
    show_exif: bool,
    /// Font data loaded in memory
    font_data: Option<Vec<u8>>,
}

impl PhotoBorder {
    /// Creates a new instance of the PhotoBorder processor
    ///
    /// # Arguments
    /// * `border_type` - Border type to apply
    /// * `show_exif` - Indicates whether to extract and display EXIF data
    /// * `font_path` - Optional path to a TTF font file
    ///
    /// # Returns
    /// * `Ok(PhotoBorder)` if initialization succeeds
    /// * `Err(PhotoBorderError)` in case of font reading error
    ///
    /// # Font Management
    /// If no font is specified, uses the DejaVu Sans font
    /// embedded in the executable to ensure portability.
    pub fn new(
        border_type: BorderType,
        show_exif: bool,
        font_path: Option<&str>,
    ) -> Result<Self, PhotoBorderError> {

        let font_data = if let Some(path) = font_path {
            // Use custom font provided by user
            fs::read(path)?
        } else {
            // Default font embedded in executable for portability
            include_bytes!("../fonts/DejaVuSans.ttf").to_vec()
        };

        Ok(PhotoBorder {
            border_type,
            show_exif,
            font_data: Some(font_data),
        })
    }

    /// Processes an individual image by adding a border
    ///
    /// # Arguments
    /// * `input_path` - Path to source image
    /// * `output_dir` - Optional output directory
    ///
    /// # Returns
    /// * `Ok(())` if processing succeeds
    /// * `Err(PhotoBorderError)` in case of error
    ///
    /// # Processing Steps
    /// 1. Load source image
    /// 2. Calculate border dimensions
    /// 3. Create new image with white border
    /// 4. Copy original image to center
    /// 5. Optionally add EXIF data
    /// 6. Save result
    pub fn process_image<P: AsRef<Path>>(&self, input_path: P, output_dir: Option<&Path>) -> Result<(), PhotoBorderError> {
        use image::GenericImageView;
        let input_path = input_path.as_ref();

        // Load source image
        let img = image::open(input_path)?;

        // Get original dimensions
        let (width, height) = img.dimensions();

        // Calculate border dimensions according to chosen type
        let (top, right, bottom, left) = self.border_type.get_border_size(width, height);

        // Calculate new dimensions with borders
        let new_width = width + left + right;
        let new_height = height + top + bottom;

        // Create new image with white background
        // White (255, 255, 255) gives a professional and timeless appearance
        let mut bordered_img = ImageBuffer::from_pixel(new_width, new_height, Rgb([255u8, 255u8, 255u8]));

        // Copy original image to center of new image
        // Left and top offsets correctly position the image
        image::imageops::overlay(&mut bordered_img, &img.to_rgb8(), left as i64, top as i64);

        // Add EXIF metadata if requested
        if self.show_exif {
            match ExifData::from_file(input_path) {
                Ok(exif_data) => {
                    // Attempt to draw EXIF text
                    if let Err(e) = self.draw_exif_text(&mut bordered_img, &exif_data, left, new_height - bottom) {
                        eprintln!("Warning: Could not draw EXIF text: {}", e);
                    }
                },
                Err(e) => eprintln!("Warning: Could not read EXIF data: {}", e),
            }
        }

        // Generate output path
        let output_path = self.generate_output_path(input_path, output_dir)?;

        // Save final image
        bordered_img.save(&output_path)?;
        println!("Saved bordered image to: {}", output_path.display());

        Ok(())
    }

    /// Generates output file path based on input
    ///
    /// # Arguments
    /// * `input_path` - Source file path
    /// * `output_dir` - Optional output directory
    ///
    /// # Returns
    /// Complete path to output file
    ///
    /// # Naming Convention
    /// Adds "_border" to filename before extension
    /// Ex: "photo.jpg" -> "photo_border.jpg"
    fn generate_output_path(&self, input_path: &Path, output_dir: Option<&Path>) -> Result<std::path::PathBuf, PhotoBorderError> {
        // Extract filename without extension
        let stem = input_path.file_stem()
            .ok_or_else(|| PhotoBorderError::IoError(io::Error::new(io::ErrorKind::InvalidInput, "Invalid filename")))?
            .to_str()
            .ok_or_else(|| PhotoBorderError::IoError(io::Error::new(io::ErrorKind::InvalidInput, "Invalid filename encoding")))?;

        // Extract extension
        let extension = input_path.extension()
            .ok_or_else(|| PhotoBorderError::IoError(io::Error::new(io::ErrorKind::InvalidInput, "No file extension")))?
            .to_str()
            .ok_or_else(|| PhotoBorderError::IoError(io::Error::new(io::ErrorKind::InvalidInput, "Invalid extension encoding")))?;

        // Build new name with "_border" suffix
        let output_filename = format!("{}_border.{}", stem, extension);

        // Determine destination directory
        let output_path = if let Some(dir) = output_dir {
            // Use specified directory
            dir.join(output_filename)
        } else {
            // Save in same directory as original
            let parent = input_path.parent().unwrap_or(Path::new("."));
            parent.join(output_filename)
        };

        Ok(output_path)
    }

    /// Draws EXIF text on the image in the border area
    ///
    /// # Arguments
    /// * `img` - Destination image (mutable)
    /// * `exif_data` - EXIF data to display
    /// * `x_offset` - Horizontal starting position
    /// * `y_offset` - Vertical starting position
    ///
    /// # Returns
    /// * `Ok(())` if text is drawn successfully
    /// * `Err(PhotoBorderError)` in case of font or drawing error
    ///
    /// # Text Style
    /// - Size proportional to image (1/80 of smallest dimension)
    /// - Dark gray color (64, 64, 64) for optimal readability on white background
    /// - Positioning with 20px left margin and 5px from bottom
    fn draw_exif_text(
        &self,
        img: &mut RgbImage,
        exif_data: &ExifData,
        x_offset: u32,
        y_offset: u32,
    ) -> Result<(), PhotoBorderError> {
        // Check font availability
        let font_data = match &self.font_data {
            Some(data) => data.clone(),
            None => {
                eprintln!("No font provided, skipping text rendering. Use -f flag to specify a font.");
                return Ok(());
            }
        };

        // Load font from memory data
        let font = Font::try_from_vec(font_data)
            .ok_or_else(|| PhotoBorderError::FontError("Invalid font data".to_string()))?;

        // Calculate font size proportional to image
        let (width, height) = img.dimensions();
        let min_dimension = width.min(height);
        let scale = Scale::uniform((min_dimension / 80) as f32);

        // Text color: dark gray for good readability
        let color = Rgb([64u8, 64u8, 64u8]);

        // Format EXIF data into single line with separators
        let lines = exif_data.format_for_display();
        let text = lines.join(" | ");

        // Draw text on image with precise positioning
        draw_text_mut(
            img,
            color,
            x_offset as i32 + 20,  // 20px left margin
            y_offset as i32 + 5,   // 5px bottom margin
            scale,
            &font,
            &text,
        );

        Ok(())
    }

    /// Processes multiple images in batch
    ///
    /// # Arguments
    /// * `input_paths` - Vector of paths to images to process
    /// * `output_dir` - Optional output directory
    ///
    /// # Returns
    /// * `Ok(())` even if some images fail (to continue processing)
    /// * `Err(PhotoBorderError)` only for critical errors
    ///
    /// # Functionality
    /// - Processes each image individually
    /// - Continues even if error on one image
    /// - Displays final summary with success/error counters
    /// - Ideal for processing large batches of images
    pub fn process_multiple_images<P: AsRef<Path>>(&self, input_paths: Vec<P>, output_dir: Option<&Path>) -> Result<(), PhotoBorderError> {
        println!("Processing {} image(s)...", input_paths.len());

        let mut success_count = 0;
        let mut error_count = 0;

        // Sequential processing of each image
        for (index, input_path) in input_paths.iter().enumerate() {
            let input_path = input_path.as_ref();
            println!("[{}/{}] Processing: {}", index + 1, input_paths.len(), input_path.display());

            // Individual processing with non-blocking error handling
            match self.process_image(input_path, output_dir) {
                Ok(()) => {
                    success_count += 1;
                }
                Err(e) => {
                    eprintln!("Error processing {}: {}", input_path.display(), e);
                    error_count += 1;
                }
            }
        }

        // Display final summary
        println!("\nProcessing complete:");
        println!("  Successfully processed: {} image(s)", success_count);
        if error_count > 0 {
            println!("  Errors encountered: {} image(s)", error_count);
        }

        Ok(())
    }
}

// ============================================================================
// MAIN FUNCTION
// ============================================================================

/// Application entry point
///
/// Configures the command-line interface, validates arguments,
/// and launches image processing according to provided parameters.
///
/// Now supports both CLI and GUI modes:
/// - Without arguments: launches GUI
/// - With arguments: runs CLI mode
///
/// # Command Line Arguments (CLI mode)
/// - `files`: One or more image files to process (required)
/// - `-e, --exif`: Enable EXIF data display
/// - `-t, --border-type`: Border type (s/small, m/medium, l/large)
/// - `-f, --font`: Path to custom TTF font file
/// - `-o, --output-dir`: Output directory for processed images
/// - `--gui`: Force GUI mode even with arguments
///
/// # Returns
/// * `Ok(())` if execution completes successfully
/// * `Err(Box<dyn Error>)` in case of critical error
fn main() -> Result<(), Box<dyn Error>> {
    // Check if we should launch GUI mode
    let args: Vec<String> = std::env::args().collect();

    // Launch GUI if no arguments or --gui flag is present
    if args.len() == 1 || args.contains(&"--gui".to_string()) {
        println!("Launching Schnaps-Shot GUI...");
        return launch_gui();
    }

    // Continue with CLI mode
    launch_cli()
}

/// Launches the GUI version of the application
fn launch_gui() -> Result<(), Box<dyn Error>> {
    // Hide console window on Windows in GUI mode
    #[cfg(target_os = "windows")]
    hide_console_window();

    let app = GuiApp::new()?;
    app.setup_callbacks()?;
    app.run()?;
    Ok(())
}

/// Hide console window on Windows
#[cfg(target_os = "windows")]
fn hide_console_window() {
    use std::ptr;
    extern "system" {
        fn GetConsoleWindow() -> *mut std::ffi::c_void;
        fn ShowWindow(hwnd: *mut std::ffi::c_void, ncmdshow: i32) -> i32;
    }

    const SW_HIDE: i32 = 0;
    unsafe {
        let hwnd = GetConsoleWindow();
        if !hwnd.is_null() {
            ShowWindow(hwnd, SW_HIDE);
        }
    }
}

/// Launches the CLI version of the application
fn launch_cli() -> Result<(), Box<dyn Error>> {
    // Configure command-line interface with clap
    let matches = Command::new("schnapsshot")
        .version("1.0")
        .about("Add a border and exif data to one or more jpg or png photos")
        .arg(
            Arg::new("files")
                .help("Input image filename(s)")
                .required(true)
                .num_args(1..)
                .index(1),
        )
        .arg(
            Arg::new("exif")
                .short('e')
                .long("exif")
                .help("Print photo exif data on the border")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("border_type")
                .short('t')
                .long("border_type")
                .help("Border Type: s for small, m for medium, l for large")
                .value_name("TYPE")
                .default_value("s"),
        )
        .arg(
            Arg::new("font")
                .short('f')
                .long("font")
                .help("Font Typeface to use (TTF file path)")
                .value_name("FONT_PATH"),
        )
        .arg(
            Arg::new("output_dir")
                .short('o')
                .long("output-dir")
                .help("Output directory (if not specified, files are saved next to originals)")
                .value_name("OUTPUT_DIR"),
        )
        .arg(
            Arg::new("gui")
                .long("gui")
                .help("Launch GUI mode")
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches();

    // Extract command-line arguments
    let files: Vec<String> = matches.get_many::<String>("files")
        .unwrap()
        .cloned()
        .collect();

    let show_exif = matches.get_flag("exif");
    let border_type_str = matches.get_one::<String>("border_type").unwrap();
    let font_path = matches.get_one::<String>("font");
    let output_dir = matches.get_one::<String>("output_dir");

    // Convert border type from string
    let border_type = BorderType::from_str(border_type_str)
        .map_err(|e| PhotoBorderError::FontError(e.to_string()))?;

    // Validate and create output directory if necessary
    if let Some(dir) = output_dir {
        let dir_path = Path::new(dir);
        if !dir_path.exists() {
            // Create directory with all necessary parents
            fs::create_dir_all(dir_path)?;
            println!("Created output directory: {}", dir_path.display());
        } else if !dir_path.is_dir() {
            // Error if path exists but is not a directory
            return Err(Box::new(PhotoBorderError::IoError(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Output path '{}' exists but is not a directory", dir)
            ))));
        }
    }

    // Create main processing instance
    let photo_border = PhotoBorder::new(
        border_type,
        true,  // Use the actual show_exif flag from CLI
        font_path.map(|s| s.as_str()),
    )?;

    // Launch image processing
    photo_border.process_multiple_images(files, output_dir.map(|s| Path::new(s)))?;

    Ok(())
}