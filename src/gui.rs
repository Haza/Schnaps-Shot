use crate::{PhotoBorder, BorderType, PhotoBorderError};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;

slint::include_modules!();

pub struct GuiApp {
    window: AppWindow,
    selected_files: Arc<Mutex<Vec<String>>>,
}

impl GuiApp {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let window = AppWindow::new()?;
        let selected_files = Arc::new(Mutex::new(Vec::new()));

        Ok(GuiApp {
            window,
            selected_files,
        })
    }

    pub fn setup_callbacks(&self) -> Result<(), Box<dyn std::error::Error>> {
        let window_weak = self.window.as_weak();
        let files = self.selected_files.clone();

        // File selection callback
        self.window.on_select_files({
            let window_weak = window_weak.clone();
            let files = files.clone();
            move || {
                if let Some(window) = window_weak.upgrade() {
                    match Self::open_file_dialog(true) {
                        Ok(selected) => {
                            if !selected.is_empty() {
                                *files.lock().unwrap() = selected.clone();
                                let files_text = if selected.len() == 1 {
                                    selected[0].clone()
                                } else {
                                    format!("{} files selected", selected.len())
                                };
                                window.set_selected_files(files_text.into());
                            }
                        }
                        Err(e) => {
                            let status = format!("Error selecting files: {}", e);
                            window.set_status_text(status.into());
                        }
                    }
                }
            }
        });

        // Output directory selection callback
        self.window.on_select_output_dir({
            let window_weak = window_weak.clone();
            move || {
                if let Some(window) = window_weak.upgrade() {
                    match Self::open_folder_dialog() {
                        Ok(Some(dir)) => {
                            window.set_output_directory(dir.into());
                        }
                        Ok(None) => {
                            // User cancelled, do nothing
                        }
                        Err(e) => {
                            let status = format!("Error selecting directory: {}", e);
                            window.set_status_text(status.into());
                        }
                    }
                }
            }
        });

        // Font selection callback
        self.window.on_select_font({
            let window_weak = window_weak.clone();
            move || {
                if let Some(window) = window_weak.upgrade() {
                    match Self::open_font_dialog() {
                        Ok(Some(font_path)) => {
                            window.set_font_path(font_path.into());
                        }
                        Ok(None) => {
                            // User cancelled, do nothing
                        }
                        Err(e) => {
                            let status = format!("Error selecting font: {}", e);
                            window.set_status_text(status.into());
                        }
                    }
                }
            }
        });

        // Process images callback
        self.window.on_process_images({
            let window_weak = window_weak.clone();
            let files = files.clone();
            move || {
                if let Some(window) = window_weak.upgrade() {
                    let files_to_process = files.lock().unwrap().clone();
                    if files_to_process.is_empty() {
                        window.set_status_text("Please select files first".into());
                        return;
                    }

                    // Get settings from UI
                    let border_type_str = window.get_border_type().to_string();
                    let show_exif = window.get_show_exif();
                    let output_dir = window.get_output_directory().to_string();
                    let font_path = window.get_font_path().to_string();

                    // Set processing state
                    window.set_processing(true);
                    window.set_status_text("Processing images...".into());

                    // Process in background thread
                    let window_weak_clone = window_weak.clone();
                    thread::spawn(move || {
                        let result = Self::process_images_background(
                            files_to_process,
                            &border_type_str,
                            show_exif,
                            if output_dir.is_empty() { None } else { Some(&output_dir) },
                            if font_path.is_empty() { None } else { Some(&font_path) },
                        );

                        // Update UI with result
                        if let Some(window) = window_weak_clone.upgrade() {
                            window.set_processing(false);
                            match result {
                                Ok(status) => {
                                    window.set_status_text(status.into());
                                }
                                Err(e) => {
                                    let error_msg = format!("Processing error: {}", e);
                                    window.set_status_text(error_msg.into());
                                }
                            }
                        }
                    });
                }
            }
        });

        Ok(())
    }

    fn process_images_background(
        files: Vec<String>,
        border_type_str: &str,
        show_exif: bool,
        output_dir: Option<&str>,
        font_path: Option<&str>,
    ) -> Result<String, PhotoBorderError> {
        // Parse border type
        let border_type = BorderType::from_str(border_type_str)
            .map_err(|e| PhotoBorderError::FontError(e.to_string()))?;

        // Create PhotoBorder instance
        let photo_border = PhotoBorder::new(border_type, show_exif, font_path)?;

        // Process images
        let mut success_count = 0;
        let mut error_count = 0;
        let mut error_details = Vec::new();

        for file_path in &files {
            match photo_border.process_image(
                file_path,
                output_dir.map(|s| Path::new(s))
            ) {
                Ok(()) => success_count += 1,
                Err(e) => {
                    error_count += 1;
                    error_details.push(format!("{}: {}", file_path, e));
                }
            }
        }

        // Build status message
        let mut status = format!("Processing complete!\nSuccessfully processed: {} image(s)", success_count);
        if error_count > 0 {
            status.push_str(&format!("\nErrors: {} image(s)", error_count));
            if !error_details.is_empty() {
                status.push_str("\n\nError details:");
                for detail in error_details.iter().take(5) { // Show first 5 errors
                    status.push_str(&format!("\n- {}", detail));
                }
                if error_details.len() > 5 {
                    status.push_str(&format!("\n... and {} more errors", error_details.len() - 5));
                }
            }
        }

        Ok(status)
    }

    fn open_file_dialog(multiple: bool) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        use std::process::Command;

        // Try to use native file dialog through system commands
        // This is a simplified implementation - in a real app you might want to use a crate like rfd
        #[cfg(target_os = "windows")]
        {
            let _used_for_other_os = multiple;
            // Windows PowerShell command for file dialog
            let output = Command::new("powershell")
                .arg("-Command")
                .arg(r#"
                Add-Type -AssemblyName System.Windows.Forms
                $openFileDialog = New-Object System.Windows.Forms.OpenFileDialog
                $openFileDialog.Filter = 'Image files (*.jpg;*.jpeg;*.png)|*.jpg;*.jpeg;*.png|All files (*.*)|*.*'
                $openFileDialog.Multiselect = $true
                $openFileDialog.Title = 'Select Images'
                if ($openFileDialog.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {
                    $openFileDialog.FileNames -join ';'
                }
                "#)
                .output()?;

            let result_string = String::from_utf8_lossy(&output.stdout);
            let result = result_string.trim();
            if result.is_empty() {
                Ok(Vec::new())
            } else {
                Ok(result.split(';').map(|s| s.to_string()).collect())
            }
        }

        #[cfg(target_os = "macos")]
        {
            // macOS osascript command for file dialog
            let script = if multiple {
                r#"tell application "System Events" to return POSIX path of (choose file with prompt "Select Images" of type {"jpg", "jpeg", "png"} with multiple selections allowed)"#
            } else {
                r#"tell application "System Events" to return POSIX path of (choose file with prompt "Select Images" of type {"jpg", "jpeg", "png"})"#
            };

            let output = Command::new("osascript")
                .arg("-e")
                .arg(script)
                .output()?;

            let output_str = String::from_utf8_lossy(&output.stdout);
            let result = output_str.trim();
            if result.is_empty() {
                Ok(Vec::new())
            } else if multiple {
                Ok(result.split(", ").map(|s| s.to_string()).collect())
            } else {
                Ok(vec![result.to_string()])
            }
        }

        #[cfg(target_os = "linux")]
        {
            // Linux zenity command for file dialog
            let mut cmd = Command::new("zenity");
            cmd.arg("--file-selection")
                .arg("--title=Select Images")
                .arg("--file-filter=Image files | *.jpg *.jpeg *.png");

            if multiple {
                cmd.arg("--multiple");
            }

            let output = cmd.output()?;

            let output_str = String::from_utf8_lossy(&output.stdout);
            let result = output_str.trim();
            if result.is_empty() {
                Ok(Vec::new())
            } else if multiple {
                Ok(result.split('|').map(|s| s.to_string()).collect())
            } else {
                Ok(vec![result.to_string()])
            }
        }

        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            // Fallback: just return empty for unsupported platforms
            eprintln!("File dialog not supported on this platform");
            Ok(Vec::new())
        }
    }

    fn open_folder_dialog() -> Result<Option<String>, Box<dyn std::error::Error>> {
        use std::process::Command;

        #[cfg(target_os = "windows")]
        {
            let output = Command::new("powershell")
                .arg("-Command")
                .arg(r#"
                Add-Type -AssemblyName System.Windows.Forms
                $folderBrowser = New-Object System.Windows.Forms.FolderBrowserDialog
                $folderBrowser.Description = 'Select Output Directory'
                if ($folderBrowser.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {
                    $folderBrowser.SelectedPath
                }
                "#)
                .output()?;

            let result_string = String::from_utf8_lossy(&output.stdout);
            let result = result_string.trim();
            Ok(if result.is_empty() { None } else { Some(result.to_string()) })
        }

        #[cfg(target_os = "macos")]
        {
            let output = Command::new("osascript")
                .arg("-e")
                .arg(r#"tell application "System Events" to return POSIX path of (choose folder with prompt "Select Output Directory")"#)
                .output()?;

            let output_str = String::from_utf8_lossy(&output.stdout);
            let result = output_str.trim();
            Ok(if result.is_empty() { None } else { Some(result.to_string()) })
        }

        #[cfg(target_os = "linux")]
        {
            let output = Command::new("zenity")
                .arg("--file-selection")
                .arg("--directory")
                .arg("--title=Select Output Directory")
                .output()?;

            let output_str = String::from_utf8_lossy(&output.stdout);
            let result = output_str.trim();
            Ok(if result.is_empty() { None } else { Some(result.to_string()) })
        }

        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            eprintln!("Folder dialog not supported on this platform");
            Ok(None)
        }
    }

    fn open_font_dialog() -> Result<Option<String>, Box<dyn std::error::Error>> {
        use std::process::Command;

        #[cfg(target_os = "windows")]
        {
            let output = Command::new("powershell")
                .arg("-Command")
                .arg(r#"
                Add-Type -AssemblyName System.Windows.Forms
                $openFileDialog = New-Object System.Windows.Forms.OpenFileDialog
                $openFileDialog.Filter = 'TrueType fonts (*.ttf)|*.ttf|OpenType fonts (*.otf)|*.otf|All files (*.*)|*.*'
                $openFileDialog.Title = 'Select Font'
                if ($openFileDialog.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {
                    $openFileDialog.FileName
                }
                "#)
                .output()?;

            let result_string = String::from_utf8_lossy(&output.stdout);
            let result = result_string.trim();
            Ok(if result.is_empty() { None } else { Some(result.to_string()) })
        }

        #[cfg(target_os = "macos")]
        {
            let output = Command::new("osascript")
                .arg("-e")
                .arg(r#"tell application "System Events" to return POSIX path of (choose file with prompt "Select Font" of type {"ttf", "otf"})"#)
                .output()?;

            let output_str = String::from_utf8_lossy(&output.stdout);
            let result = output_str.trim();
            Ok(if result.is_empty() { None } else { Some(result.to_string()) })
        }

        #[cfg(target_os = "linux")]
        {
            let output = Command::new("zenity")
                .arg("--file-selection")
                .arg("--title=Select Font")
                .arg("--file-filter=Font files | *.ttf *.otf")
                .output()?;

            let output_str = String::from_utf8_lossy(&output.stdout);
            let result = output_str.trim();
            Ok(if result.is_empty() { None } else { Some(result.to_string()) })
            Ok(if result.is_empty() { None } else { Some(result.to_string()) })
        }

        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            eprintln!("Font dialog not supported on this platform");
            Ok(None)
        }
    }

    pub fn run(&self) -> Result<(), slint::PlatformError> {
        self.window.run()
    }
}