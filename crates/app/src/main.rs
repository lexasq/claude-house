//! Claude House window and tray. A thin shell over `claude_house_ops`.
// Release builds have no console window.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use claude_house_ops::{self as ops, CopyStatus, HandlerOwner, House, Overview};
use claude_house_core::account::SignIn;
use serde::Serialize;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, State, WindowEvent, Wry};

const TRAY_ID: &str = "main";

/// Sign-in state of one Claude copy.
#[derive(Serialize)]
struct CopyView {
    /// "never-opened", "signed-out", "signed-in" or "unreadable".
    sign_in: &'static str,
    /// Names of the other copies on the same account ("Main account" for the stock copy).
    same_account_as: Vec<String>,
}

#[derive(Serialize)]
struct ProfileView {
    id: String,
    name: String,
    color: String,
    isolate_code: bool,
    #[serde(flatten)]
    copy: CopyView,
}

#[derive(Serialize)]
struct OverviewView {
    main: CopyView,
    profiles: Vec<ProfileView>,
}

fn overview_view(o: &Overview) -> OverviewView {
    let copy = |c: &CopyStatus| CopyView {
        sign_in: match c.sign_in {
            SignIn::NeverOpened => "never-opened",
            SignIn::SignedOut => "signed-out",
            SignIn::SignedIn(_) => "signed-in",
            SignIn::Unreadable => "unreadable",
        },
        same_account_as: o.names(&c.same_account_as),
    };
    OverviewView {
        main: copy(&o.main),
        profiles: o
            .profiles
            .iter()
            .map(|(p, c)| ProfileView {
                id: p.id.as_str().to_owned(),
                name: p.name.as_str().to_owned(),
                color: p.color.as_str().to_owned(),
                isolate_code: p.isolate_code,
                copy: copy(c),
            })
            .collect(),
    }
}

#[derive(Serialize)]
struct RoutingView {
    /// The watcher thread is running.
    enabled: bool,
    /// Who claude:// goes to right now: "claude-house", "claude-desktop" or "other".
    owner: &'static str,
    /// Why the watcher stopped on its own, if it did.
    error: Option<String>,
}

/// The one thread allowed to write the claude:// handler. Held behind a mutex: at most one exists.
struct Router {
    stop: Arc<AtomicBool>,
    thread: JoinHandle<()>,
}

struct AppState {
    house: House,
    router: Mutex<Option<Router>>,
    router_error: Mutex<Option<String>>,
}

#[tauri::command]
fn overview(state: State<AppState>) -> Result<OverviewView, String> {
    Ok(overview_view(&state.house.overview()?))
}

#[tauri::command]
fn launch_main(state: State<AppState>) -> Result<(), String> {
    state.house.launch_main()
}

#[tauri::command]
fn add_profile(
    app: AppHandle,
    state: State<AppState>,
    id: String,
    name: String,
    color: String,
    isolate_code: bool,
) -> Result<String, String> {
    let name = (!name.trim().is_empty()).then_some(name.as_str());
    let profile = state.house.add(&id, name, &color, isolate_code)?;
    refresh_tray(&app);
    Ok(profile.name.as_str().to_owned())
}

/// Returns the data folder, which is kept.
#[tauri::command]
fn remove_profile(app: AppHandle, state: State<AppState>, id: String) -> Result<String, String> {
    let kept = state.house.remove(&id)?;
    refresh_tray(&app);
    Ok(kept.display().to_string())
}

#[tauri::command]
fn launch_profile(state: State<AppState>, id: String) -> Result<(), String> {
    state.house.launch(&id)
}

#[tauri::command]
fn routing_status(state: State<AppState>) -> RoutingView {
    routing_view(&state)
}

/// Runs off the UI thread: turning routing off waits up to a second for the watcher to stop.
/// The choice is remembered for the next start; routing is never turned on without the user.
#[tauri::command(async)]
fn set_routing(app: AppHandle, state: State<AppState>, enabled: bool) -> Result<RoutingView, String> {
    let marker = routing_marker(&state.house);
    if enabled {
        std::fs::create_dir_all(&state.house.root).map_err(|e| e.to_string())?;
        std::fs::write(&marker, b"").map_err(|e| format!("cannot save the setting: {e}"))?;
        start_routing(&app);
    } else {
        match std::fs::remove_file(&marker) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(format!("cannot save the setting: {e}")),
            _ => {}
        }
        stop_routing(&app)?;
    }
    Ok(routing_view(&state))
}

/// Present when the user turned login routing on.
fn routing_marker(house: &House) -> std::path::PathBuf {
    house.root.join("routing-enabled")
}

fn routing_view(state: &AppState) -> RoutingView {
    let enabled = state.router.lock().unwrap().as_ref().is_some_and(|r| !r.thread.is_finished());
    let owner = match state.house.handler_owner() {
        HandlerOwner::ClaudeHouse => "claude-house",
        HandlerOwner::ClaudeDesktop => "claude-desktop",
        HandlerOwner::Other(_) => "other",
    };
    RoutingView { enabled, owner, error: state.router_error.lock().unwrap().clone() }
}

/// Starts the watcher unless one is already running. Running it twice changes nothing.
fn start_routing(app: &AppHandle) {
    let state = app.state::<AppState>();
    let mut router = state.router.lock().unwrap();
    if router.as_ref().is_some_and(|r| !r.thread.is_finished()) {
        return;
    }
    *state.router_error.lock().unwrap() = None;
    let stop = Arc::new(AtomicBool::new(false));
    let (thread_stop, thread_app) = (stop.clone(), app.clone());
    let thread = std::thread::spawn(move || {
        let result = ops::keep_handler(&thread_stop, |_| {
            let _ = thread_app.emit("routing-changed", ());
        });
        if let Err(e) = result {
            *thread_app.state::<AppState>().router_error.lock().unwrap() = Some(e);
        }
        let _ = thread_app.emit("routing-changed", ());
    });
    *router = Some(Router { stop, thread });
}

/// Stops the watcher and waits for it, so it cannot take claude:// back, then returns claude:// to Claude.
fn stop_routing(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let router = state.router.lock().unwrap().take();
    if let Some(r) = router {
        r.stop.store(true, Ordering::Relaxed);
        let _ = r.thread.join();
    }
    state.house.restore_handler()
}

fn tray_menu(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let menu = Menu::new(app)?;
    menu.append(&MenuItem::with_id(app, "show", "Open Claude House", true, None::<&str>)?)?;
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItem::with_id(app, "launch-main", "Open Main Claude", true, None::<&str>)?)?;
    let profiles = app.state::<AppState>().house.profiles().unwrap_or_default();
    for p in profiles.iter() {
        let label = format!("Open {}", p.name.as_str());
        menu.append(&MenuItem::with_id(app, format!("launch:{}", p.id.as_str()), label, true, None::<&str>)?)?;
    }
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?)?;
    Ok(menu)
}

fn refresh_tray(app: &AppHandle) {
    if let (Some(tray), Ok(menu)) = (app.tray_by_id(TRAY_ID), tray_menu(app)) {
        let _ = tray.set_menu(Some(menu));
    }
}

fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

/// Gives claude:// back to Claude only if this app was routing, so quitting never touches the registry otherwise.
fn quit(app: &AppHandle) {
    if app.state::<AppState>().router.lock().unwrap().is_some() {
        let _ = stop_routing(app);
    }
    app.exit(0);
}

fn on_menu(app: &AppHandle, id: &str) {
    match id {
        "show" => show_main(app),
        "quit" => quit(app),
        _ => {
            let house = &app.state::<AppState>().house;
            let result = match id.strip_prefix("launch:") {
                Some(profile) => house.launch(profile),
                None if id == "launch-main" => house.launch_main(),
                None => Ok(()),
            };
            if let Err(e) = result {
                let _ = app.emit("error", e);
            }
        }
    }
}

fn main() {
    let house = House::from_process().expect("APPDATA and LOCALAPPDATA are always set on Windows");
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| show_main(app)))
        .manage(AppState { house, router: Mutex::new(None), router_error: Mutex::new(None) })
        .invoke_handler(tauri::generate_handler![
            overview,
            launch_main,
            add_profile,
            remove_profile,
            launch_profile,
            routing_status,
            set_routing
        ])
        .setup(|app| {
            let handle = app.handle();
            TrayIconBuilder::with_id(TRAY_ID)
                .icon(app.default_window_icon().expect("bundle icon is configured").clone())
                .tooltip("Claude House")
                .menu(&tray_menu(handle)?)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| on_menu(app, event.id.as_ref()))
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                        show_main(tray.app_handle());
                    }
                })
                .build(app)?;
            if routing_marker(&handle.state::<AppState>().house).exists() {
                start_routing(handle);
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing the window keeps Claude House in the tray, so login routing keeps working.
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .run(tauri::generate_context!())
        .expect("Claude House failed to start");
}
