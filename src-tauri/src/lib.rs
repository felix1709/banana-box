mod app_state;
mod command_auth;
mod commands;
mod db;
mod desktop_state;
mod daily_tasks;
mod fs_atomic;
mod legacy_import;
mod library;
mod migration;
mod projects;
mod provider_http;
mod providers;
mod safe_archive;
mod secrets;
mod startup;
mod window_state;

use app_state::{
    AppOperationGate, AppServices, RestoreBlockerRegistry, StartupGate, StartupStatus,
};
use command_auth::{FloatArgs, MainOrFloatArgs};
use desktop_state::{
    clamp_float_position, logical_to_physical, physical_to_saved, select_restore_monitor,
    DesktopStateStore, LogicalPoint, MonitorWorkArea, PhysicalPoint, PhysicalRect,
    FLOAT_WINDOW_LOGICAL_SIZE,
};
use legacy_import::BackupStagingCoordinator;
use migration::{StartupCoordinator, StartupOutcome};
use secrets::{CredentialMutationCoordinator, WindowsCredentialStore};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex,
};
use std::time::{Duration, Instant};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Manager, PhysicalPosition, WebviewWindow, WindowEvent};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use window_state::{
    PanelRevealAck, PanelStateSnapshot, PanelTargetChanged, PanelTransitionReason,
    WindowStateService,
};

struct MainWindowDragState {
    ignore_focus_loss_until: Mutex<Option<Instant>>,
}

struct MainWindowPinState {
    pinned: Mutex<bool>,
}

struct FloatPositionState {
    store: DesktopStateStore,
    save_generation: AtomicU64,
    monitor_signature: Mutex<Vec<String>>,
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
                        request_panel_toggle(app, PanelTransitionReason::Shortcut);
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
                    request_panel_visibility(
                        window.app_handle(),
                        false,
                        PanelTransitionReason::FocusLoss,
                    );
                }
            }

            if window.label() == "main" {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    request_panel_visibility(
                        window.app_handle(),
                        false,
                        PanelTransitionReason::TitlebarClose,
                    );
                }
            }

            if window.label() == "floatbtn"
                && matches!(
                    event,
                    WindowEvent::Moved(_) | WindowEvent::ScaleFactorChanged { .. }
                )
            {
                schedule_float_position_save(window.app_handle());
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
            commands::read_import_dir,
            commands::download_image,
            commands::check_for_update,
            commands::provider_commands::list_ai_providers,
            commands::provider_commands::save_ai_provider,
            commands::provider_commands::clear_ai_provider_credential,
            commands::provider_commands::check_ai_provider_connection,
            commands::provider_commands::reverse_image_prompt,
            commands::backup_commands::inspect_legacy_import,
            commands::backup_commands::commit_legacy_import,
            commands::backup_commands::discard_legacy_import_preview,
            commands::import_image_from_path,
            commands::compress_media,
            commands::suggest_compressed_output_path,
            projects::list_projects,
            projects::create_project,
            projects::update_project,
            projects::save_project_with_stages,
            projects::set_project_stage,
            projects::archive_project,
            projects::delete_project,
            begin_main_window_drag,
            begin_main_window_resize,
            set_main_window_pinned,
            get_panel_state,
            ack_panel_reveal,
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
    app.manage(BackupStagingCoordinator::new(&data_dir)?);

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
        app.manage(Arc::new(WindowStateService::default()));
        setup_ready_desktop_shell(app, &data_dir)?;
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

fn setup_ready_desktop_shell(
    app: &tauri::App,
    data_dir: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    app.state::<StartupGate>().require_ready()?;
    restore_float_button(app, data_dir)?;

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
                request_panel_toggle(tray.app_handle(), PanelTransitionReason::Tray);
            }
        })
        .menu(
            &tauri::menu::MenuBuilder::new(app)
                .item(&tauri::menu::MenuItemBuilder::with_id("show", "显示").build(app)?)
                .item(&tauri::menu::MenuItemBuilder::with_id("quit", "退出").build(app)?)
                .build()?,
        )
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => request_panel_toggle(app, PanelTransitionReason::Tray),
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;

    start_monitor_topology_watcher(app.handle().clone());

    Ok(())
}

fn restore_float_button(
    app: &tauri::App,
    data_dir: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let window = app
        .get_webview_window("floatbtn")
        .ok_or("float button window not found")?;
    let monitors = monitor_work_areas(&window)?;
    let store = DesktopStateStore::new(data_dir.join("desktop-state.json"));
    let saved = store.load()?;
    let monitor = saved
        .as_ref()
        .map(|saved| select_restore_monitor(saved, &monitors))
        .or_else(|| monitors.iter().find(|monitor| monitor.primary))
        .or_else(|| monitors.first())
        .ok_or("no monitor is available for the float button")?;
    let window_size = logical_pixels(FLOAT_WINDOW_LOGICAL_SIZE, monitor.scale_factor);
    let clamp_margin = logical_pixels(12.0, monitor.scale_factor);
    let requested = saved.as_ref().map_or_else(
        || PhysicalPoint {
            x: monitor.bounds.x + monitor.bounds.width as i32
                - window_size
                - logical_pixels(16.0, monitor.scale_factor),
            y: monitor.bounds.y + (monitor.bounds.height as i32 - window_size) / 2,
        },
        |saved| {
            logical_to_physical(
                LogicalPoint {
                    x: saved.logical_x,
                    y: saved.logical_y,
                },
                monitor.bounds,
                monitor.scale_factor,
            )
        },
    );
    let restored = clamp_float_position(requested, monitor.bounds, window_size, clamp_margin);
    window.set_position(PhysicalPosition::new(restored.x, restored.y))?;
    store.save(&physical_to_saved(restored, monitor))?;

    let signature = monitor_signature(&monitors);
    app.manage(FloatPositionState {
        store,
        save_generation: AtomicU64::new(0),
        monitor_signature: Mutex::new(signature),
    });
    window.show()?;
    Ok(())
}

fn logical_pixels(value: f64, scale_factor: f64) -> i32 {
    (value * scale_factor).round() as i32
}

fn monitor_work_areas(window: &WebviewWindow) -> Result<Vec<MonitorWorkArea>, String> {
    let primary_id = window
        .primary_monitor()
        .map_err(|error| error.to_string())?
        .as_ref()
        .map(monitor_id);
    window
        .available_monitors()
        .map_err(|error| error.to_string())?
        .iter()
        .map(|monitor| {
            let work_area = monitor.work_area();
            let id = monitor_id(monitor);
            Ok(MonitorWorkArea {
                primary: primary_id.as_ref().is_some_and(|primary| primary == &id),
                id,
                bounds: PhysicalRect {
                    x: work_area.position.x,
                    y: work_area.position.y,
                    width: work_area.size.width,
                    height: work_area.size.height,
                },
                scale_factor: monitor.scale_factor(),
            })
        })
        .collect()
}

fn monitor_id(monitor: &tauri::Monitor) -> String {
    monitor.name().cloned().unwrap_or_else(|| {
        let position = monitor.position();
        let size = monitor.size();
        format!(
            "{}:{}:{}:{}",
            position.x, position.y, size.width, size.height
        )
    })
}

fn monitor_signature(monitors: &[MonitorWorkArea]) -> Vec<String> {
    let mut signature: Vec<_> = monitors
        .iter()
        .map(|monitor| {
            format!(
                "{}:{}:{}:{}:{}:{}",
                monitor.id,
                monitor.bounds.x,
                monitor.bounds.y,
                monitor.bounds.width,
                monitor.bounds.height,
                monitor.scale_factor,
            )
        })
        .collect();
    signature.sort();
    signature
}

fn schedule_float_position_save(app: &tauri::AppHandle) {
    let app = app.clone();
    let Some(state) = app.try_state::<FloatPositionState>() else {
        return;
    };
    let generation = state.save_generation.fetch_add(1, Ordering::SeqCst) + 1;
    drop(state);

    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(250));
        let Some(state) = app.try_state::<FloatPositionState>() else {
            return;
        };
        if state.save_generation.load(Ordering::SeqCst) != generation {
            return;
        }
        drop(state);
        let _ = persist_float_position(&app);
    });
}

fn persist_float_position(app: &tauri::AppHandle) -> Result<(), String> {
    let state = app
        .try_state::<FloatPositionState>()
        .ok_or_else(|| "float position state is unavailable".to_string())?;
    let window = app
        .get_webview_window("floatbtn")
        .ok_or_else(|| "float button window not found".to_string())?;
    let monitors = monitor_work_areas(&window)?;
    let current_id = window
        .current_monitor()
        .map_err(|error| error.to_string())?
        .as_ref()
        .map(monitor_id);
    let monitor = current_id
        .as_ref()
        .and_then(|id| monitors.iter().find(|monitor| monitor.id == *id))
        .or_else(|| monitors.iter().find(|monitor| monitor.primary))
        .or_else(|| monitors.first())
        .ok_or_else(|| "no monitor is available for the float button".to_string())?;
    let position = window.outer_position().map_err(|error| error.to_string())?;
    state.store.save(&physical_to_saved(
        PhysicalPoint {
            x: position.x,
            y: position.y,
        },
        monitor,
    ))
}

fn normalize_float_position(app: &tauri::AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("floatbtn")
        .ok_or_else(|| "float button window not found".to_string())?;
    let monitors = monitor_work_areas(&window)?;
    let current_id = window
        .current_monitor()
        .map_err(|error| error.to_string())?
        .as_ref()
        .map(monitor_id);
    let monitor = current_id
        .as_ref()
        .and_then(|id| monitors.iter().find(|monitor| monitor.id == *id))
        .or_else(|| monitors.iter().find(|monitor| monitor.primary))
        .or_else(|| monitors.first())
        .ok_or_else(|| "no monitor is available for the float button".to_string())?;
    let position = window.outer_position().map_err(|error| error.to_string())?;
    let clamped = clamp_float_position(
        PhysicalPoint {
            x: position.x,
            y: position.y,
        },
        monitor.bounds,
        logical_pixels(FLOAT_WINDOW_LOGICAL_SIZE, monitor.scale_factor),
        logical_pixels(12.0, monitor.scale_factor),
    );
    if clamped.x != position.x || clamped.y != position.y {
        window
            .set_position(PhysicalPosition::new(clamped.x, clamped.y))
            .map_err(|error| error.to_string())?;
    }
    persist_float_position(app)
}

fn start_monitor_topology_watcher(app: tauri::AppHandle) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(5));
        let Some(window) = app.get_webview_window("floatbtn") else {
            continue;
        };
        let Ok(monitors) = monitor_work_areas(&window) else {
            continue;
        };
        let signature = monitor_signature(&monitors);
        let changed = app
            .try_state::<FloatPositionState>()
            .and_then(|state| {
                state.monitor_signature.lock().ok().map(|mut previous| {
                    if *previous == signature {
                        false
                    } else {
                        *previous = signature;
                        true
                    }
                })
            })
            .unwrap_or(false);
        if changed {
            let _ = normalize_float_position(&app);
        }
    });
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

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct EmptyPanelCommandArgs {}

fn panel_service(app: &tauri::AppHandle) -> Result<Arc<WindowStateService>, String> {
    app.try_state::<Arc<WindowStateService>>()
        .map(|service| Arc::clone(&service))
        .ok_or_else(|| "STARTUP_NOT_READY".to_string())
}

#[tauri::command]
fn get_panel_state(
    app: tauri::AppHandle,
    gate: tauri::State<StartupGate>,
    _args: FloatArgs<EmptyPanelCommandArgs>,
) -> Result<PanelStateSnapshot, String> {
    gate.require_ready()?;
    Ok(panel_service(&app)?.snapshot())
}

#[tauri::command]
fn ack_panel_reveal(
    app: tauri::AppHandle,
    gate: tauri::State<StartupGate>,
    acknowledgement: FloatArgs<PanelRevealAck>,
) -> Result<(), String> {
    gate.require_ready()?;
    panel_service(&app).and_then(|service| {
        service
            .acknowledge_reveal(&acknowledgement.0)
            .then_some(())
            .ok_or_else(|| "STALE_PANEL_GENERATION".to_string())
    })
}

#[tauri::command]
fn toggle_panel(
    app: tauri::AppHandle,
    gate: tauri::State<StartupGate>,
    _args: FloatArgs<EmptyPanelCommandArgs>,
) -> Result<PanelTargetChanged, String> {
    gate.require_ready()?;
    panel_service(&app)?.toggle(&app, PanelTransitionReason::Banana)
}

#[tauri::command]
fn show_panel(
    app: tauri::AppHandle,
    gate: tauri::State<StartupGate>,
    _args: MainOrFloatArgs<EmptyPanelCommandArgs>,
) -> Result<PanelTargetChanged, String> {
    gate.require_ready()?;
    panel_service(&app)?.request_visibility(&app, true, PanelTransitionReason::FileDrop)
}

fn request_panel_toggle(app: &tauri::AppHandle, reason: PanelTransitionReason) {
    if let Ok(service) = panel_service(app) {
        let _ = service.toggle(app, reason);
    }
}

fn request_panel_visibility(
    app: &tauri::AppHandle,
    target_visible: bool,
    reason: PanelTransitionReason,
) {
    if let Ok(service) = panel_service(app) {
        let _ = service.request_visibility(app, target_visible, reason);
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
