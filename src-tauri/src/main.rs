mod commands;
mod converter;
mod file_utils;

use crate::commands::PendingOpenFiles;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{image::Image, Emitter, Listener, LogicalSize, Manager, WindowEvent};

const TRAY_ICON_BYTES: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/icons/tray.png"));

fn load_tray_icon() -> tauri::Result<Image<'static>> {
    Image::from_bytes(TRAY_ICON_BYTES)
}

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.center();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn hide_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
}

fn handle_finder_opened_files(app: &tauri::AppHandle, urls: &[tauri::Url]) {
    let files: Vec<String> = urls
        .iter()
        .filter_map(|url| url.to_file_path().ok())
        .map(|path| path.to_string_lossy().to_string())
        .collect();

    if files.is_empty() {
        return;
    }

    println!("[rust:info] finder opened {} files", files.len());

    let pending_open_files = app.state::<PendingOpenFiles>();
    if let Ok(mut pending) = pending_open_files.0.lock() {
        *pending = files.clone();
    }

    show_main_window(app);

    if let Some(window) = app.get_webview_window("main") {
        let _ = window.emit("finder-open-files", &files);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(PendingOpenFiles::default())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_image_info,
            commands::convert_images,
            commands::play_system_sound,
            commands::take_pending_open_files,
            commands::estimate_output,
            commands::open_save_dialog,
            commands::get_parent_dir,
            commands::debug_log,
            commands::sync_window_viewport,
        ])
        .setup(|app| {
            println!("[rust:info] setup start");

            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_size(LogicalSize::new(428.0, 318.0));

                match window.inner_size() {
                    Ok(size) => println!("[rust:info] inner size = {}x{}", size.width, size.height),
                    Err(error) => eprintln!("[rust:error] failed to read inner size: {error}"),
                }

                match window.outer_size() {
                    Ok(size) => println!("[rust:info] outer size = {}x{}", size.width, size.height),
                    Err(error) => eprintln!("[rust:error] failed to read outer size: {error}"),
                }
            }

            let show = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;

            let tray_icon = load_tray_icon()?;

            TrayIconBuilder::with_id("main")
                .icon(tray_icon)
                .icon_as_template(true)
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        println!("[rust:info] tray menu show");
                        show_main_window(app)
                    }
                    "quit" => std::process::exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        println!("[rust:info] tray left click");
                        show_main_window(tray.app_handle());
                    }
                })
                .build(app)?;

            let app_handle = app.handle().clone();
            app.listen("conversion_completed", move |_| {
                println!("[rust:info] conversion completed event");
                hide_main_window(&app_handle);
            });

            println!("[rust:info] setup complete");

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                println!("[rust:info] window close intercepted");
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building PixShrink")
        .run(|app, event| {
            if let tauri::RunEvent::Opened { urls } = event {
                handle_finder_opened_files(app, &urls);
            }
        });
}

fn main() {
    run();
}
