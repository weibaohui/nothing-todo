use std::sync::{Arc, Mutex};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, WindowEvent,
};

mod backend;

pub struct AppState {
    pub backend_port: Mutex<u16>,
}

#[tauri::command]
fn get_backend_url(state: tauri::State<Arc<AppState>>) -> String {
    let port = *state.backend_port.lock().expect("backend_port mutex poisoned");
    format!("http://localhost:{}", port)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Create the shared state before setup
    let app_state = Arc::new(AppState {
        backend_port: Mutex::new(0),
    });

    // Clone for the spawned task
    let state_for_task = app_state.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_focus();
            }
        }))
        .manage(app_state)
        .setup(move |_app| {
            // Spawn backend using Tauri's async runtime to avoid "runtime within runtime" panic
            tauri::async_runtime::spawn(async move {
                match backend::spawn_backend().await {
                    Ok(port) => {
                        *state_for_task.backend_port.lock().expect("backend_port mutex poisoned") = port;
                        log::info!("Backend spawned on port {}", port);
                    }
                    Err(e) => {
                        log::error!("Failed to spawn backend: {}", e);
                    }
                }
            });

            Ok(())
        })
        .setup(|app| {
            // System tray
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let show_item = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

            let _tray = TrayIconBuilder::new()
                .menu(&menu)
                .tooltip("ntd")
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => app.exit(0),
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        if let Some(window) = tray.app_handle().get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            // Close to tray
            if let Some(main_window) = app.get_webview_window("main") {
                let window_clone = main_window.clone();
                main_window.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = window_clone.hide();
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_backend_url])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
