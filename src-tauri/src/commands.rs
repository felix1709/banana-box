// src-tauri/src/commands.rs
// IPC 命令：前端通过 src/lib/ipc.ts 调用这些函数。
// 所有系统操作（文件、剪贴板）在此封装，前端不直接碰系统。

use crate::library::{self, Library};
use std::path::PathBuf;
use tauri::Manager;

fn data_dir(app: &tauri::AppHandle) -> PathBuf {
    app.path()
        .app_data_dir()
        .expect("no app data dir")
}

#[tauri::command]
pub fn load_library(app: tauri::AppHandle) -> Library {
    library::load_library(&data_dir(&app))
}

#[tauri::command]
pub fn save_library(app: tauri::AppHandle, library: Library) -> Result<(), String> {
    library::save_library(&data_dir(&app), &library).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn copy_to_clipboard(app: tauri::AppHandle, text: String) -> Result<(), String> {
    use tauri_plugin_clipboard_manager::ClipboardExt;
    app.clipboard().write_text(text).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_image(app: tauri::AppHandle, bytes: Vec<u8>, ext: String) -> Result<String, String> {
    let dir = data_dir(&app).join("images");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let id = uuid::Uuid::new_v4().simple().to_string();
    let name = format!("{}.{}", id, ext);
    let path = dir.join(&name);
    std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;
    Ok(format!("images/{}", name))
}

#[tauri::command]
pub fn delete_image(app: tauri::AppHandle, path: String) -> Result<(), String> {
    let full = data_dir(&app).join(&path);
    if full.exists() {
        std::fs::remove_file(&full).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn read_image_bytes(app: tauri::AppHandle, path: String) -> Result<Vec<u8>, String> {
    let full = data_dir(&app).join(&path);
    std::fs::read(&full).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn export_library(app: tauri::AppHandle, dest: String) -> Result<(), String> {
    let zip_path = std::path::PathBuf::from(&dest);
    library::export_library(&data_dir(&app), &zip_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn import_library(app: tauri::AppHandle, zip_path: String) -> Result<Library, String> {
    library::import_library(std::path::Path::new(&zip_path), &data_dir(&app))
        .map_err(|e| e.to_string())
}
