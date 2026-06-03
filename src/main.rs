use std::io::Write;

use tiny_skia::Color;
use wisp::draw::{color_from_hex, rgba_to_bgra};
use wisp::input::InputAction;
use wisp_components::list_view::ListColors;

use omni_app_launcher::config::Config;
use omni_app_launcher::draw::draw_launcher_frame;
use omni_app_launcher::state::{self, OmniApp};
use omni_calculator::{draw_calculator, CalcState};
use omni_clipboard::{draw_clipboard, ClipboardState};
use omni_daemon::shortcuts::ShortcutAction;
use omni_daemon::Daemon;
use omni_ipc::IpcCommand;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: jemallocator::Jemalloc = jemallocator::Jemalloc;

enum ActiveApp {
    Launcher(Box<OmniApp>),
    Calculator(CalcState),
    Clipboard(ClipboardState),
}

fn resize_for_app(daemon: &mut Daemon, app: &ActiveApp, config: &Config) -> (i32, i32) {
    let (w, h) = match app {
        ActiveApp::Launcher(_) | ActiveApp::Clipboard(_) => {
            let row_h = omni_app_launcher::draw::compute_row_height(
                daemon.font_system_mut(),
                config.font.size as f32,
                &config.font.family,
            );
            let padding = 10.0f32;
            let header_h = 42.0f32;
            let footer_h = 28.0f32;
            let gap = 8.0f32;
            let body_h = (6.0 * row_h).round();
            let total_h = (padding + header_h + gap + body_h + gap + footer_h + padding).round() as i32;
            (config.window.width, total_h)
        }
        ActiveApp::Calculator(_) => {
            let pad = 14.0;
            let label_h = 22.0;
            let input_h = 42.0;
            let gap = 14.0;
            let result_h = 30.0;
            let footer_h = 28.0;
            let bottom_pad = 10.0;
            let h = (pad + label_h + input_h + gap + result_h + gap + footer_h + bottom_pad) as i32;
            (config.window.width, h)
        }
    };
    let (cw, ch) = daemon.surface_size();
    if w != cw || h != ch {
        daemon.resize_surface(w, h);
        daemon.poll();
    }
    (w, h)
}

fn draw_frame(daemon: &mut Daemon, active: &mut ActiveApp, config: &Config, cursor_visible: bool) {
    let mouse_y = if daemon.mouse_inside() { daemon.mouse_y() } else { -9999.0 };
    daemon.draw_frame(|surf, font, swash| {
        match active {
            ActiveApp::Launcher(launcher) => {
                draw_launcher_frame(
                    &mut surf.pixmap, font, swash, launcher, config, cursor_visible, mouse_y,
                );
            }
            ActiveApp::Calculator(calc) => {
                draw_calculator(
                    &mut surf.pixmap, font, swash,
                    color_from_hex(&config.theme.bg),
                    color_from_hex(&config.theme.fg),
                    color_from_hex(&config.theme.badge_destructive_fg),
                    color_from_hex(&config.theme.desc_fg),
                    color_from_hex(&config.theme.entry_bg),
                    color_from_hex(&config.theme.border),
                    &config.font.family, config.theme.border_radius as f32, calc, cursor_visible,
                );
            }
            ActiveApp::Clipboard(clipboard_state) => {
                let list_colors = ListColors {
                    bg: color_from_hex(&config.theme.bg),
                    hover: {
                        let mut c = color_from_hex(&config.theme.selected_bg);
                        c = Color::from_rgba8(
                            (c.red() * 255.0) as u8,
                            (c.green() * 255.0) as u8,
                            (c.blue() * 255.0) as u8,
                            50,
                        );
                        c
                    },
                    active_bg: color_from_hex(&config.theme.selected_bg),
                    active_border: color_from_hex(&config.theme.border),
                    active_highlight: false,
                    fg: color_from_hex(&config.theme.fg),
                    selected_fg: color_from_hex(&config.theme.selected_fg),
                    desc_fg: color_from_hex(&config.theme.desc_fg),
                    selected_desc_fg: color_from_hex(&config.theme.selected_fg),
                };
                draw_clipboard(
                    &mut surf.pixmap, font, swash,
                    color_from_hex(&config.theme.bg),
                    &list_colors,
                    color_from_hex(&config.theme.entry_bg),
                    color_from_hex(&config.theme.border),
                    config.font.size as f32,
                    color_from_hex(&config.theme.placeholder_fg),
                    color_from_hex(&config.theme.caret),
                    &config.font.family,
                    config.theme.border_radius as f32,
                    clipboard_state,
                    cursor_visible,
                    mouse_y,
                );
            }
        }
        if let Some(mmap) = surf.mmap.as_mut() {
            rgba_to_bgra(surf.pixmap.data(), unsafe {
                std::slice::from_raw_parts_mut(mmap.as_mut_ptr(), surf.pixmap.data().len())
            });
        }
    });
    daemon.commit();
}

fn main() {
    use tracing_subscriber::EnvFilter;

    let args: Vec<String> = std::env::args().collect();
    let is_daemon = args.iter().any(|a| a == "--daemon");

    if is_daemon {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
        let log_dir = std::path::PathBuf::from(&home).join(".local/share/omni");
        std::fs::create_dir_all(&log_dir).ok();
        let log_path = log_dir.join("daemon.log");
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .expect("failed to open log file");
        tracing_subscriber::fmt()
            .with_writer(std::sync::Mutex::new(file))
            .with_env_filter(
                EnvFilter::builder()
                    .with_default_directive(tracing::level_filters::LevelFilter::INFO.into())
                    .parse("info,omni_app_launcher=debug,omni=debug")
                    .unwrap(),
            )
            .init();
        tracing::info!("=== omni daemon started ===");
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(
                EnvFilter::builder()
                    .with_default_directive(tracing::level_filters::LevelFilter::INFO.into())
                    .parse("info,omni_app_launcher=debug,omni=debug")
                    .unwrap(),
            )
            .init();
    }

    let config = Config::load();

    if is_daemon {
        loop {
            run_daemon(config.clone());
            tracing::info!("daemon exited, restarting in 500ms...");
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
    } else {
        run_client(&args, &config);
    }
}

fn run_daemon(config: Config) {
    tracing::info!("daemon starting");
    let mut daemon = Daemon::connect();
    daemon.init_globals();
    daemon.init_ipc();

    let mut shortcuts: Vec<omni_daemon::sway_backend::ShortcutEntry> = Vec::new();
    if let Some(keys) = &config.shortcuts.launcher {
        shortcuts.push(omni_daemon::sway_backend::ShortcutEntry {
            id: "launcher".into(),
            keys: keys.clone(),
        });
    }
    if let Some(keys) = &config.shortcuts.calculator {
        shortcuts.push(omni_daemon::sway_backend::ShortcutEntry {
            id: "calculator".into(),
            keys: keys.clone(),
        });
    }
    if let Some(keys) = &config.shortcuts.clipboard {
        shortcuts.push(omni_daemon::sway_backend::ShortcutEntry {
            id: "clipboard".into(),
            keys: keys.clone(),
        });
    }
    for (app_id, keys) in &config.shortcuts.apps {
        shortcuts.push(omni_daemon::sway_backend::ShortcutEntry {
            id: format!("app:{}", app_id),
            keys: keys.clone(),
        });
    }
    if !shortcuts.is_empty() {
        omni_daemon::sway_backend::register_shortcuts(&shortcuts);
    }

    let row_h = omni_app_launcher::draw::compute_row_height(
        daemon.font_system_mut(),
        config.font.size as f32,
        &config.font.family,
    );
    let padding = 10.0f32;
    let header_h = 42.0f32;
    let footer_h = 28.0f32;
    let gap = 8.0f32;
    let body_h = (6.0 * row_h).round();
    let total_h = (padding + header_h + gap + body_h + gap + footer_h + padding).round() as i32;
    daemon.create_surface(config.window.width, total_h);
    if let Err(e) = daemon.conn.flush() {
        tracing::error!("wayland flush: {e}");
        return;
    }
    daemon.poll();
    daemon.hide_surface();

    let mut active: Option<ActiveApp> = None;

    std::panic::set_hook(Box::new(|panic_info| {
        tracing::error!("PANIC: {}", panic_info);
    }));

    while daemon.running() {
        if !daemon.poll() { break; }

        for cmd in daemon.drain_ipc_actions() {
            match cmd {
                IpcCommand::Launcher => {
                    let app = ActiveApp::Launcher(Box::new(OmniApp::new(&config)));
                    let (w, h) = resize_for_app(&mut daemon, &app, &config);
                    daemon.show_surface(w, h);
                    active = Some(app);
                }
                IpcCommand::Calculator(expr) => {
                    let calc = ActiveApp::Calculator(CalcState::new(&expr.unwrap_or_default()));
                    let (w, h) = resize_for_app(&mut daemon, &calc, &config);
                    daemon.show_surface(w, h);
                    active = Some(calc);
                }
                IpcCommand::Quit => {
                    daemon.quit();
                }
                IpcCommand::Clipboard => {
                    let entries = daemon.clipboard_entries();
                    let clipboard_state = ClipboardState::with_entries(entries);
                    let app = ActiveApp::Clipboard(clipboard_state);
                    let (w, h) = resize_for_app(&mut daemon, &app, &config);
                    daemon.show_surface(w, h);
                    active = Some(app);
                }
            }
        }

        if let Some(shortcut) = daemon.drain_shortcut_command() {
            match shortcut {
                ShortcutAction::ShowLauncher => {
                    let app = ActiveApp::Launcher(Box::new(OmniApp::new(&config)));
                    let (w, h) = resize_for_app(&mut daemon, &app, &config);
                    daemon.show_surface(w, h);
                    active = Some(app);
                }
                ShortcutAction::ShowCalculator => {
                    let calc = ActiveApp::Calculator(CalcState::new(""));
                    let (w, h) = resize_for_app(&mut daemon, &calc, &config);
                    daemon.show_surface(w, h);
                    active = Some(calc);
                }
                ShortcutAction::ShowClipboard => {
                    let entries = daemon.clipboard_entries();
                    let clipboard_state = ClipboardState::with_entries(entries);
                    let app = ActiveApp::Clipboard(clipboard_state);
                    let (w, h) = resize_for_app(&mut daemon, &app, &config);
                    daemon.show_surface(w, h);
                    active = Some(app);
                }
            }
        }

        let mut hide = false;
        let mut switch_to = None;

        if let Some(ref mut cur) = active {
            match cur {
                ActiveApp::Launcher(app) => {
                    let pointer = daemon.pointer().cloned();
                    let cursor = if daemon.mouse_inside() {
                        app.mouse.cursor_at(daemon.mouse_y())
                    } else {
                        wisp::CursorStyle::Arrow
                    };
                    if let Some(ref p) = pointer {
                        daemon.cursor().set_cursor(p, cursor);
                    }

                    let row_height = omni_app_launcher::draw::compute_row_height(
                        daemon.font_system_mut(),
                        config.font.size as f32,
                        &config.font.family,
                    );
                    let delta = daemon.drain_scroll(row_height);
                    if delta != 0.0
                        && app.mouse.region_at(daemon.mouse_y(), daemon.mouse_y()) == Some(wisp::events::RegionId::Body)
                    {
                        app.list_state.scroll.scroll_by(-delta);
                        daemon.set_dirty();
                    }

                    for action in daemon.drain_actions() {
                        let body_h = app.mouse.body_bounds().map_or(0.0, |(_, h)| h);
                        match action {
                            InputAction::Confirm => {
                                if !app.results.is_empty() {
                                    let exec = app.results[app.selected_index].entry.exec.clone();
                                    OmniApp::launch_selected(&exec);
                                }
                                hide = true;
                            }
                            InputAction::Cancel => {
                                hide = true;
                            }
                            _ => {
                                let row_height = omni_app_launcher::draw::compute_row_height(
                                    daemon.font_system_mut(),
                                    config.font.size as f32,
                                    &config.font.family,
                                );
                                state::handle_action(app, action, row_height, body_h);
                                daemon.set_dirty();
                                daemon.mark_input();
                            }
                        }
                    }

                    if let Some(action) = daemon.process_repeat() {
                        let body_h = app.mouse.body_bounds().map_or(0.0, |(_, h)| h);
                        let row_height = omni_app_launcher::draw::compute_row_height(
                            daemon.font_system_mut(),
                            config.font.size as f32,
                            &config.font.family,
                        );
                        state::handle_action(app, action, row_height, body_h);
                        daemon.set_dirty();
                        daemon.mark_input();
                    }

                    if app.query().starts_with('=') {
                        let expr = app.query()[1..].to_string();
                        app.search("");
                        daemon.set_dirty();
                        switch_to = Some(ActiveApp::Calculator(CalcState::new(&expr)));
                    }
                }
                ActiveApp::Calculator(calc) => {
                    for action in daemon.drain_actions() {
                        match action {
                            InputAction::Cancel => {
                                hide = true;
                            }
                            InputAction::Confirm => {
                                calc.evaluate();
                                daemon.set_dirty();
                            }
                            InputAction::AppendChar(ch) => {
                                calc.append_char(ch);
                                daemon.set_dirty();
                                daemon.mark_input();
                            }
                            InputAction::Backspace => {
                                calc.backspace();
                                daemon.set_dirty();
                                daemon.mark_input();
                            }
                            InputAction::DeleteWordBackward | InputAction::DeleteWordForward => {
                                calc.clear();
                                daemon.set_dirty();
                                daemon.mark_input();
                            }
                            _ => {}
                        }
                    }

                    if let Some(action) = daemon.process_repeat() {
                        match action {
                            InputAction::Backspace => {
                                calc.backspace();
                                daemon.set_dirty();
                                daemon.mark_input();
                            }
                            InputAction::AppendChar(ch) => {
                                calc.append_char(ch);
                                daemon.set_dirty();
                                daemon.mark_input();
                            }
                            _ => {}
                        }
                    }
                }
                ActiveApp::Clipboard(clipboard_state) => {
                    for action in daemon.drain_actions() {
                        match action {
                            InputAction::Confirm => {
                                if let Some(text) = clipboard_state.selected() {
                                    daemon.clipboard_paste(&text);
                                }
                                hide = true;
                            }
                            InputAction::Cancel => {
                                hide = true;
                            }
                            InputAction::SelectNext => {
                                clipboard_state.select_next();
                                daemon.set_dirty();
                            }
                            InputAction::SelectPrev => {
                                clipboard_state.select_prev();
                                daemon.set_dirty();
                            }
                            InputAction::AppendChar(ch) => {
                                clipboard_state.append_char(ch);
                                daemon.set_dirty();
                                daemon.mark_input();
                            }
                            InputAction::Backspace => {
                                clipboard_state.backspace();
                                daemon.set_dirty();
                                daemon.mark_input();
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        if let Some(new_app) = switch_to {
            let (w, h) = resize_for_app(&mut daemon, &new_app, &config);
            daemon.show_surface(w, h);
            active = Some(new_app);
        } else if hide {
            daemon.hide_surface();
            active = None;
        }

        if let Some(ref mut cur) = active {
            let cursor_visible = daemon.cursor_visible();
            draw_frame(&mut daemon, cur, &config, cursor_visible);
            daemon.clear_dirty();
        }
    }
    tracing::info!("daemon shutting down");
}

fn run_client(args: &[String], config: &Config) {
    if let Some(pos) = args.iter().position(|a| a == "--shortcut") {
        let id = args.get(pos + 1).expect("missing shortcut id");
        handle_shortcut(id, config);
        return;
    }

    let mut socket = match omni_ipc::connect_abstract("omni-ipc") {
        Ok(s) => s,
        Err(_) => {
            eprintln!("omni daemon not running. Start it with: omni --daemon");
            std::process::exit(1);
        }
    };

    let cmd = if args.iter().any(|a| a == "--clipboard") {
        "clipboard\n".to_string()
    } else if args.iter().any(|a| a == "--calculator" || a == "-c") {
        if args.len() > 2 && !args[2].starts_with('-') {
            format!("calculator:{}\n", args[2])
        } else {
            "calculator\n".to_string()
        }
    } else if args.iter().any(|a| a == "--quit") {
        "quit\n".to_string()
    } else {
        "launcher\n".to_string()
    };

    let _ = socket.write_all(cmd.as_bytes());
}

fn handle_shortcut(id: &str, config: &Config) {
    match id {
        "launcher" => {
            let mut socket = ipc_connect();
            let _ = socket.write_all(b"launcher\n");
        }
        "calculator" => {
            let mut socket = ipc_connect();
            let _ = socket.write_all(b"calculator\n");
        }
        "clipboard" => {
            let mut socket = ipc_connect();
            let _ = socket.write_all(b"clipboard\n");
        }
        app_id => {
            let clean_id = app_id.strip_prefix("app:").unwrap_or(app_id);
            launch_desktop_app(clean_id, config);
        }
    }
}

fn ipc_connect() -> std::os::unix::net::UnixStream {
    match omni_ipc::connect_abstract("omni-ipc") {
        Ok(s) => s,
        Err(_) => {
            eprintln!("omni daemon not running. Start it with: omni --daemon");
            std::process::exit(1);
        }
    }
}

fn launch_desktop_app(app_id: &str, _config: &Config) {
    let mut app_index = omni_search::app_index::AppIndex::new();
    if app_index.refresh().is_err() {
        return;
    }
    if let Some(entry) = app_index.apps().iter().find(|a| a.id == app_id || a.exec_name.as_deref() == Some(app_id)) {
        omni_app_launcher::state::OmniApp::launch_selected(&entry.exec);
    }
}
