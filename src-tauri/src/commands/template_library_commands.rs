use crate::{app_state::StartupGate, template_library};
use tauri::Manager;

#[tauri::command]
pub fn load_template_library(
    app: tauri::AppHandle,
    gate: tauri::State<'_, StartupGate>,
) -> Result<template_library::TemplateLibraryDto, String> {
    gate.require_ready()?;
    let resource_dir = app.path().resource_dir().map_err(|error| error.to_string())?;
    template_library::load_template_library(&resource_dir)
}

#[tauri::command]
pub fn load_template_image(
    app: tauri::AppHandle,
    gate: tauri::State<'_, StartupGate>,
    image_path: String,
) -> Result<Vec<u8>, String> {
    gate.require_ready()?;
    let resource_dir = app.path().resource_dir().map_err(|error| error.to_string())?;
    template_library::load_template_image(&resource_dir, &image_path)
}
