mod app_state;
mod command_auth;
mod commands;
mod db;
mod fs_atomic;
mod library;
mod migration;
mod provider_http;
mod providers;
mod secrets;
mod startup;

use app_state::{
    AppOperationGate, AppServices, RestoreBlockerRegistry, StartupGate, StartupStatus,
};
use migration::{StartupCoordinator, StartupOutcome};
use secrets::{CredentialMutationCoordinator, WindowsCredentialStore};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Manager, WindowEvent};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

struct MainWindowDragState {
    ignore_focus_loss_until: Mutex<Option<Instant>>,
}

struct MainWindowPinState {
    pinned: Mutex<bool>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(MainWindowDragState {
            ignore_focus_loss_until: Mutex::new(None),
        })
        .manage(MainWindowPinState {
            pinned: Mutex::new(false),
        })
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        toggle_main_panel(app);
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .setup(
            |app: &mut tauri::App| -> Result<(), Box<dyn std::error::Error>> {
                initialize_startup(app)
            },
        )
        .on_window_event(|window, event| {
            // 只有主面板失焦隐藏；悬浮按钮常驻
            if matches!(event, WindowEvent::Resized(_)) && window.label() == "main" {
                if let Some(state) = window.app_handle().try_state::<MainWindowDragState>() {
                    protect_main_window_interaction(&state);
                }
            }
            if let WindowEvent::Focused(focused) = event {
                let drag_ignore_until = window
                    .app_handle()
                    .try_state::<MainWindowDragState>()
                    .and_then(|state| {
                        state
                            .ignore_focus_loss_until
                            .lock()
                            .ok()
                            .and_then(|guard| *guard)
                    });
                let main_window_pinned = window
                    .app_handle()
                    .try_state::<MainWindowPinState>()
                    .and_then(|state| state.pinned.lock().ok().map(|guard| *guard))
                    .unwrap_or(false);
                if should_hide_main_on_focus_loss(
                    window.label(),
                    *focused,
                    drag_ignore_until,
                    Instant::now(),
                    main_window_pinned,
                ) {
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::startup_commands::get_startup_status,
            commands::startup_commands::acknowledge_migration_summary,
            commands::load_library,
            commands::save_library,
            commands::copy_to_clipboard,
            commands::save_image,
            commands::delete_image,
            commands::read_image_bytes,
            commands::export_library,
            commands::import_library,
            commands::read_import_dir,
            commands::download_image,
            commands::check_for_update,
            commands::provider_commands::list_ai_providers,
            commands::provider_commands::save_ai_provider,
            commands::provider_commands::clear_ai_provider_credential,
            commands::provider_commands::check_ai_provider_connection,
            commands::provider_commands::reverse_image_prompt,
            commands::import_image_from_path,
            commands::compress_media,
            commands::suggest_compressed_output_path,
            begin_main_window_drag,
            begin_main_window_resize,
            set_main_window_pinned,
            toggle_panel,
            show_panel,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn initialize_startup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let data_dir = app.path().app_data_dir()?;
    let operations = Arc::new(AppOperationGate::default());
    let restore_blockers = Arc::new(RestoreBlockerRegistry::default());
    let credential_mutations = Arc::new(CredentialMutationCoordinator::default());

    app.manage(operations.clone());
    app.manage(restore_blockers);

    let coordinator = StartupCoordinator::new(
        Arc::new(WindowsCredentialStore),
        credential_mutations,
        operations,
    );
    let (startup_gate, services, show_main_window) =
        split_startup_outcome(coordinator.run(&data_dir));
    app.manage(startup_gate);

    if let Some(services) = services {
        app.manage(services);
        setup_ready_desktop_shell(app)?;
    }

    if show_main_window {
        if let Some(window) = app.get_webview_window("main") {
            window.show()?;
            window.set_focus()?;
        }
    }

    Ok(())
}

fn split_startup_outcome(outcome: StartupOutcome) -> (StartupGate, Option<AppServices>, bool) {
    match outcome {
        StartupOutcome::Ready {
            services,
            migration_summary,
        } => (
            StartupGate::new(StartupStatus::Ready { migration_summary }),
            Some(services),
            false,
        ),
        StartupOutcome::Recovery(info) => (
            StartupGate::new(StartupStatus::Recovery {
                message: info.message,
                backup_paths: info.backup_paths,
            }),
            None,
            true,
        ),
    }
}

fn setup_ready_desktop_shell(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let shortcut = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyB);
    app.global_shortcut().register(shortcut)?;

    TrayIconBuilder::with_id("main")
        .tooltip("Banana Box")
        .icon(app.default_window_icon().expect("icon").clone())
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle_main_panel(tray.app_handle());
            }
        })
        .menu(
            &tauri::menu::MenuBuilder::new(app)
                .item(&tauri::menu::MenuItemBuilder::with_id("show", "显示").build(app)?)
                .item(&tauri::menu::MenuItemBuilder::with_id("quit", "退出").build(app)?)
                .build()?,
        )
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => toggle_main_panel(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;

    Ok(())
}

#[tauri::command]
fn begin_main_window_drag(
    app: tauri::AppHandle,
    state: tauri::State<MainWindowDragState>,
) -> Result<(), String> {
    protect_main_window_interaction(&state);
    let win = app
        .get_webview_window("main")
        .ok_or_else(|| "main window not found".to_string())?;
    win.start_dragging().map_err(|err| err.to_string())
}

#[tauri::command]
fn begin_main_window_resize(state: tauri::State<MainWindowDragState>) {
    protect_main_window_interaction(&state);
}

fn protect_main_window_interaction(state: &MainWindowDragState) {
    if let Ok(mut ignore_until) = state.ignore_focus_loss_until.lock() {
        *ignore_until = Some(Instant::now() + Duration::from_millis(1200));
    }
}

#[tauri::command]
fn set_main_window_pinned(
    state: tauri::State<MainWindowPinState>,
    pinned: bool,
) -> Result<(), String> {
    let mut guard = state.pinned.lock().map_err(|err| err.to_string())?;
    *guard = pinned;
    Ok(())
}

#[tauri::command]
fn toggle_panel(app: tauri::AppHandle) {
    toggle_main_panel(&app);
}

#[tauri::command]
fn show_panel(app: tauri::AppHandle) {
    show_main_panel(&app);
}

fn toggle_main_panel(app: &tauri::AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        if win.is_visible().unwrap_or(false) {
            let _ = win.hide();
        } else {
            let _ = win.show();
            let _ = win.set_focus();
        }
    }
}

fn show_main_panel(app: &tauri::AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.set_focus();
    }
}

fn should_hide_main_on_focus_loss(
    label: &str,
    focused: bool,
    drag_ignore_until: Option<Instant>,
    now: Instant,
    pinned: bool,
) -> bool {
    label == "main" && !focused && !pinned && drag_ignore_until.is_none_or(|until| now >= until)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn main_window_focus_loss_hides_when_not_dragging() {
        let now = Instant::now();

        assert!(should_hide_main_on_focus_loss(
            "main", false, None, now, false
        ));
    }

    #[test]
    fn main_window_focus_loss_does_not_hide_during_drag_grace_period() {
        let now = Instant::now();

        assert!(!should_hide_main_on_focus_loss(
            "main",
            false,
            Some(now + Duration::from_millis(800)),
            now,
            false,
        ));
    }

    #[test]
    fn main_window_focus_loss_does_not_hide_when_pinned() {
        let now = Instant::now();

        assert!(!should_hide_main_on_focus_loss(
            "main", false, None, now, true
        ));
    }

    #[test]
    fn recovery_startup_does_not_expose_business_services_and_shows_the_main_window() {
        let (startup_gate, services, show_main_window) = split_startup_outcome(
            crate::migration::StartupOutcome::Recovery(crate::migration::RecoveryInfo {
                message: "Local data needs recovery.".into(),
                backup_paths: vec!["C:\\backup\\library-v0.json".into()],
            }),
        );

        assert!(services.is_none());
        assert!(show_main_window);
        assert!(matches!(
            startup_gate.status().unwrap(),
            crate::app_state::StartupStatus::Recovery { backup_paths, .. }
                if backup_paths == ["C:\\backup\\library-v0.json"]
        ));
    }
}
