use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, WindowEvent,
};

mod backend;

pub struct AppState {
    pub backend_url: Mutex<String>,
}

#[tauri::command]
fn get_backend_url(state: tauri::State<Arc<AppState>>) -> String {
    state.backend_url.lock().expect("backend_url mutex poisoned").clone()
}

const INSTALL_PAGE: &str = r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>ntd - 安装</title>
  <style>
    * { margin: 0; padding: 0; box-sizing: border-box; }
    body {
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
      background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
      min-height: 100vh;
      display: flex;
      align-items: center;
      justify-content: center;
    }
    .container {
      background: white;
      border-radius: 16px;
      padding: 48px;
      max-width: 500px;
      text-align: center;
      box-shadow: 0 25px 50px -12px rgba(0,0,0,0.25);
    }
    h1 { color: #1a1a2e; margin-bottom: 16px; font-size: 28px; }
    p { color: #64748b; margin-bottom: 24px; line-height: 1.6; }
    .command {
      background: #1a1a2e;
      color: #a5f3fc;
      padding: 16px 24px;
      border-radius: 8px;
      font-family: 'SF Mono', Monaco, monospace;
      font-size: 14px;
      margin-bottom: 24px;
      word-break: break-all;
    }
    .step { display: flex; align-items: center; gap: 16px; margin-bottom: 16px; text-align: left; }
    .step-num {
      width: 32px; height: 32px; border-radius: 50%;
      background: #667eea; color: white;
      display: flex; align-items: center; justify-content: center;
      font-weight: bold; flex-shrink: 0;
    }
    .step-text { color: #334155; }
    .hint { color: #94a3b8; font-size: 14px; margin-top: 24px; }
  </style>
</head>
<body>
  <div class="container">
    <h1>欢迎使用 ntd</h1>
    <p>请按照以下步骤安装：</p>
    <div class="step">
      <div class="step-num">1</div>
      <div class="step-text">安装 ntd</div>
    </div>
    <div class="command">npm install -g @weibaohui/nothing-todo@latest</div>
    <div class="step">
      <div class="step-num">2</div>
      <div class="step-text">启动 ntd 服务</div>
    </div>
    <div class="command">ntd daemon start</div>
    <p class="hint">安装并启动后，关闭此窗口重新打开即可。</p>
  </div>
</body>
</html>
"#;

const LOADING_PAGE: &str = r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>ntd - 启动中</title>
  <style>
    * { margin: 0; padding: 0; box-sizing: border-box; }
    body {
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
      background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
      min-height: 100vh;
      display: flex;
      align-items: center;
      justify-content: center;
    }
    .container {
      background: white;
      border-radius: 16px;
      padding: 48px;
      max-width: 400px;
      text-align: center;
      box-shadow: 0 25px 50px -12px rgba(0,0,0,0.25);
    }
    .spinner {
      width: 40px;
      height: 40px;
      border: 4px solid #e2e8f0;
      border-top-color: #667eea;
      border-radius: 50%;
      animation: spin 1s linear infinite;
      margin: 0 auto 24px;
    }
    @keyframes spin { to { transform: rotate(360deg); } }
    h1 { color: #1a1a2e; margin-bottom: 8px; font-size: 24px; }
    p { color: #64748b; }
  </style>
</head>
<body>
  <div class="container">
    <div class="spinner"></div>
    <h1>正在启动...</h1>
    <p>请稍候</p>
  </div>
</body>
</html>
"#;

fn show_html(window: &tauri::WebviewWindow, html: &str) {
    let escaped = html
        .replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace('\n', "\\n");
    if let Err(e) = window.eval(&format!("document.write('{}'); document.close()", escaped)) {
        log::error!("Failed to show HTML: {}", e);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app_state = Arc::new(AppState {
        backend_url: Mutex::new(String::new()),
    });

    let state_for_setup = app_state.clone();

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
        .setup(move |app| {
            let main_window = app
                .get_webview_window("main")
                .expect("main window not found");

            tauri::async_runtime::spawn(async move {
                match backend::check_ntd_status().await {
                    backend::NtdStatus::NotInstalled => {
                        log::info!("ntd not installed, showing install page");
                        show_html(&main_window, INSTALL_PAGE);
                    }
                    backend::NtdStatus::Installed { running: true, port } => {
                        let url = format!("http://localhost:{}", port);
                        *state_for_setup.backend_url.lock().expect("backend_url mutex poisoned") = url.clone();
                        log::info!("ntd already running on {}, redirecting", port);
                        if let Err(e) = main_window.eval(&format!("window.location.href = '{}'", url)) {
                            log::error!("Failed to redirect: {}", e);
                        }
                    }
                    backend::NtdStatus::Installed { running: false, port } => {
                        log::info!("ntd installed but not running, starting daemon on port {}", port);
                        show_html(&main_window, LOADING_PAGE);

                        match backend::start_ntd_daemon(port).await {
                            Ok(port) => {
                                let url = format!("http://localhost:{}", port);
                                *state_for_setup.backend_url.lock().expect("backend_url mutex poisoned") = url.clone();
                                tokio::time::sleep(Duration::from_secs(1)).await;
                                if let Err(e) = main_window.eval(&format!("window.location.href = '{}'", url)) {
                                    log::error!("Failed to redirect: {}", e);
                                }
                            }
                            Err(e) => {
                                log::error!("Failed to start ntd daemon: {}", e);
                            }
                        }
                    }
                }
            });

            Ok(())
        })
        .setup(|app| {
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