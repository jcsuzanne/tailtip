mod index;
mod scan;
mod settings;
mod theme;
mod v3;
mod v4;

use scan::Project;
use settings::Settings;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, State, WindowEvent};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
use tauri_plugin_positioner::{Position, WindowExt};
use index::{Hit, Index, Loader};
use scan::ConfigFile;
use theme::{Kind, Theme};

#[derive(Default)]
struct AppState {
    settings: Mutex<Settings>,
    projects: Mutex<Option<Vec<Project>>>,
    /// Themes of every config, for the project view and cross-project search.
    index: Arc<Index>,
    node: Mutex<Option<Option<String>>>,
    /// Keep the popover open while a native dialog has focus.
    keep_open: AtomicBool,
}

fn settings_path(app: &AppHandle) -> PathBuf {
    app.path().app_config_dir().unwrap_or_else(|_| PathBuf::from(".")).join("settings.json")
}

fn projects_cache_path(app: &AppHandle) -> PathBuf {
    app.path().app_cache_dir().unwrap_or_else(|_| PathBuf::from(".")).join("projects.json")
}

fn resolver_path(app: &AppHandle) -> PathBuf {
    let bundled = app
        .path()
        .resource_dir()
        .map(|d| d.join("resolver/resolve.cjs"))
        .ok()
        .filter(|p| p.is_file());
    bundled.unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("resolver/dist/resolve.cjs"))
}

fn node_binary(state: &AppState) -> Option<String> {
    if let Some(explicit) = state.settings.lock().unwrap().node_path.clone().filter(|p| !p.is_empty()) {
        return Some(explicit);
    }
    let mut cached = state.node.lock().unwrap();
    cached.get_or_insert_with(v3::detect_node).clone()
}

fn loader(app: &AppHandle, state: &AppState) -> Loader {
    Loader { node: node_binary(state), resolver: resolver_path(app) }
}

fn build_index(app: &AppHandle, projects: Vec<Project>) {
    let state = app.state::<AppState>();
    state.index.build(app.clone(), loader(app, &state), projects);
}

fn do_scan(app: &AppHandle, state: &AppState) -> Vec<Project> {
    let roots = state.settings.lock().unwrap().roots.clone();
    let projects = scan::scan(&roots);
    let path = projects_cache_path(app);
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(&path, serde_json::to_string(&projects).unwrap_or_default());
    *state.projects.lock().unwrap() = Some(projects.clone());
    build_index(app, projects.clone());
    projects
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

#[tauri::command]
fn get_settings(state: State<AppState>) -> Settings {
    state.settings.lock().unwrap().clone()
}

#[tauri::command]
fn save_settings(app: AppHandle, state: State<AppState>, settings: Settings) -> Result<(), String> {
    let previous_shortcut = state.settings.lock().unwrap().shortcut.clone();
    settings.save(&settings_path(&app)).map_err(|e| e.to_string())?;
    if settings.shortcut != previous_shortcut {
        register_shortcut(&app, &settings.shortcut, Some(&previous_shortcut))?;
    }
    *state.node.lock().unwrap() = None;
    *state.settings.lock().unwrap() = settings;
    Ok(())
}

#[tauri::command]
async fn list_projects(app: AppHandle) -> Vec<Project> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        if let Some(p) = state.projects.lock().unwrap().clone() {
            return p;
        }
        let cached: Option<Vec<Project>> = std::fs::read_to_string(projects_cache_path(&app))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok());
        match cached {
            Some(p) => {
                *state.projects.lock().unwrap() = Some(p.clone());
                build_index(&app, p.clone());
                p
            }
            None => do_scan(&app, &state),
        }
    })
    .await
    .unwrap_or_default()
}

#[tauri::command]
async fn rescan(app: AppHandle) -> Vec<Project> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        do_scan(&app, &state)
    })
    .await
    .unwrap_or_default()
}

#[tauri::command]
async fn load_theme(app: AppHandle, path: String, kind: Kind) -> Theme {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        state.index.get(&loader(&app, &state), &ConfigFile { path, kind })
    })
    .await
    .unwrap_or_else(|e| Theme::failed(kind, "", e.to_string()))
}

#[tauri::command]
fn search_tokens(state: State<AppState>, query: String) -> Vec<Hit> {
    let projects = state.projects.lock().unwrap().clone().unwrap_or_default();
    let hidden = state.settings.lock().unwrap().hidden.clone();
    state.index.search(&projects, &hidden, &query)
}

#[tauri::command]
fn open_in_editor(path: String, line: Option<u32>) -> Result<(), String> {
    let target = match line {
        Some(l) => format!("{path}:{l}"),
        None => path.clone(),
    };
    let url = format!("cursor://file{}", target.replace(' ', "%20"));
    let ok = Command::new("open").arg(&url).status().map(|s| s.success()).unwrap_or(false);
    if !ok {
        Command::new("open").args(["-a", "Cursor", &path]).status().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn reveal_in_finder(path: String) -> Result<(), String> {
    Command::new("open").args(["-R", &path]).status().map(|_| ()).map_err(|e| e.to_string())
}

#[tauri::command]
fn hide_window(app: AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.hide();
    }
}

#[tauri::command]
fn set_keep_open(state: State<AppState>, keep: bool) {
    state.keep_open.store(keep, Ordering::SeqCst);
}

#[tauri::command]
fn detected_node(state: State<AppState>) -> Option<String> {
    node_binary(&state)
}

#[tauri::command]
fn quit(app: AppHandle) {
    app.exit(0);
}

// ---------------------------------------------------------------------------
// Window / tray / shortcut
// ---------------------------------------------------------------------------

fn toggle_window(app: &AppHandle, from_tray: bool) {
    let Some(window) = app.get_webview_window("main") else { return };
    if window.is_visible().unwrap_or(false) {
        let _ = window.hide();
        return;
    }
    let placed = from_tray && window.move_window(Position::TrayBottomCenter).is_ok();
    if !placed {
        let _ = window.move_window(Position::TopRight);
    }
    let _ = window.show();
    let _ = window.set_focus();
    let _ = app.emit("popover-shown", ());
}

fn register_shortcut(app: &AppHandle, accelerator: &str, previous: Option<&str>) -> Result<(), String> {
    let gs = app.global_shortcut();
    if let Some(prev) = previous.and_then(|p| p.parse::<Shortcut>().ok()) {
        let _ = gs.unregister(prev);
    }
    if accelerator.trim().is_empty() {
        return Ok(());
    }
    let shortcut: Shortcut = accelerator.parse().map_err(|e| format!("Raccourci invalide : {e}"))?;
    gs.register(shortcut).map_err(|e| format!("Raccourci indisponible : {e}"))
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        toggle_window(app, false);
                    }
                })
                .build(),
        )
        .manage(AppState::default())
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let handle = app.handle().clone();
            let settings = Settings::load(&settings_path(&handle));
            if let Err(e) = register_shortcut(&handle, &settings.shortcut, None) {
                eprintln!("[tailtip] {e}");
            }
            *app.state::<AppState>().settings.lock().unwrap() = settings;

            let rescan_item = MenuItem::with_id(app, "rescan", "Rescanner les projets", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quitter TailTip", true, Some("Cmd+Q"))?;
            let separator = PredefinedMenuItem::separator(app)?;
            let menu = Menu::with_items(app, &[&rescan_item, &separator, &quit_item])?;

            TrayIconBuilder::with_id("main")
                .icon(Image::from_bytes(include_bytes!("../icons/tray.png"))?)
                .icon_as_template(true)
                .tooltip("TailTip")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => app.exit(0),
                    "rescan" => {
                        let _ = app.emit("rescan-requested", ());
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    tauri_plugin_positioner::on_tray_event(tray.app_handle(), &event);
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        toggle_window(tray.app_handle(), true);
                    }
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::Focused(false) = event {
                let keep = window.app_handle().state::<AppState>().keep_open.load(Ordering::SeqCst);
                if !keep {
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_settings,
            save_settings,
            list_projects,
            rescan,
            load_theme,
            search_tokens,
            open_in_editor,
            reveal_in_finder,
            hide_window,
            set_keep_open,
            detected_node,
            quit
        ])
        .run(tauri::generate_context!())
        .expect("error while running TailTip");
}

#[cfg(test)]
mod probe {
    /// Cross-project search on real folders: `cargo test probe_search -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn probe_search() {
        let roots = crate::settings::Settings::default().roots;
        let projects = crate::scan::scan(&roots);
        let loader = crate::index::Loader {
            node: crate::v3::detect_node(),
            resolver: std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("resolver/dist/resolve.cjs"),
        };
        let index = crate::index::Index::default();
        let t = std::time::Instant::now();
        let mut errors = 0;
        for c in projects.iter().flat_map(|p| p.configs.iter()) {
            let theme = index.get(&loader, c);
            if let Some(e) = theme.error {
                errors += 1;
                println!("ERR {}: {}", c.path, e);
            }
        }
        println!("indexed sequentially in {:?}, {errors} errors", t.elapsed());
        for q in ["ivory", "cognac", "#f9af26", "outer-x"] {
            let hits = index.search(&projects, &[], q);
            println!("{q}: {} hits", hits.len());
            for h in hits.iter().take(4) {
                println!("   {} {}.{} = {}", h.project, h.group, h.token.name, h.token.value);
            }
        }
    }

    /// Manual check against real folders: `cargo test probe -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn probe_real_roots() {
        let roots = crate::settings::Settings::default().roots;
        let t = std::time::Instant::now();
        let projects = crate::scan::scan(&roots);
        println!("{} projects in {:?} (roots: {:?})", projects.len(), t.elapsed(), roots);
        for p in &projects {
            for c in &p.configs {
                if c.kind == crate::theme::Kind::V4 {
                    let theme = crate::v4::parse(std::path::Path::new(&c.path));
                    let summary: Vec<String> = theme.groups.iter().map(|g| format!("{}:{}", g.key, g.tokens.len())).collect();
                    println!("v4 {:<28} prefix={:?} v={:?} {} [{}]", p.name, theme.prefix, theme.version, c.path.rsplit('/').next().unwrap(), summary.join(" "));
                } else {
                    println!("v3 {:<28} {}", p.name, c.path.rsplit('/').next().unwrap());
                }
            }
        }
    }
}
