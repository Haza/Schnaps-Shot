fn main() {
    // Compile Slint UI
    slint_build::compile("ui/app.slint").unwrap();

    // Add Windows-specific build configuration
    #[cfg(target_os = "windows")]
    {
        // This ensures proper Windows subsystem handling
        println!("cargo:rerun-if-changed=build.rs");

        // You could add Windows resource compilation here if needed
        // For example, to add an application icon:
        // winres::WindowsResource::new()
        //     .set_icon("icon.ico")
        //     .compile().unwrap();
    }
}