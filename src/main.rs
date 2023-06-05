#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use std::fs::create_dir_all;
use std::path::PathBuf;
use std::sync::Mutex;

use anyhow::Error;
use chrono::Local;
use dirs::home_dir;
use walkdir::WalkDir;

pub const APP_NAME: &str = "template_native_app";

fn load_icon(data: &[u8]) -> eframe::IconData {
    let (icon_rgba, icon_width, icon_height) = {
        let image = image::load_from_memory(data)
            .expect("Failed to open icon path")
            .into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };

    eframe::IconData {
        rgba: icon_rgba,
        width: icon_width,
        height: icon_height,
    }
}

fn create_log_folder_and_cleanup() -> PathBuf {
    let process_create_dir = || -> Result<PathBuf, Error> {
        let mut save_path = PathBuf::new();
        let date = Local::now().format("%Y-%m-%d").to_string();
        if let Some(home_dir) = home_dir() {
            save_path.push(&home_dir);
        }
        save_path.push(APP_NAME);
        save_path.push(&date);
        let log_path = save_path.join("logs");

        create_dir_all(&log_path)?;

        // Old log files cleanup
        let mut vec = Vec::new();
        for file in WalkDir::new(&save_path).into_iter().filter_map(|e| e.ok()) {
            if file.file_type().is_file() {
                if let Some(ext) = file.path().extension() {
                    if ext == "log" {
                        vec.push(file.path().to_path_buf());
                    }
                }
            }
        }
        vec.sort();
        vec.reverse();
        while vec.len() > 20 {
            trash::delete(vec.pop().expect("Error deleting log file.")).unwrap();
        }

        Ok(log_path)
    };
    if process_create_dir().is_err() {
        return PathBuf::new();
    }
    process_create_dir().unwrap()
}

fn main() -> eframe::Result<()> {
    // Create log file
    let log_path = create_log_folder_and_cleanup();
    let log_file = log_path.join(format!("{}_{}.log", APP_NAME, Local::now().format("%Y-%m-%d_%H-%M-%S-%f")));
    let log_file = std::fs::File::create(log_file).unwrap();
    let log_file = Mutex::new(log_file);

    tracing_subscriber::fmt()
        .with_writer(log_file)
        .init();

    log_panics::init();

    let icon = include_bytes!("resources/icon.png");

    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "eframe template",
        native_options,
        Box::new(|cc| Box::new(eframe_template::TemplateApp::new(cc))),
    )
}
