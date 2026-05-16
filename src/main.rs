use wisp::draw::{color_from_hex, rgba_to_bgra};
use wisp::input::InputAction;

use omni_app_launcher::config::Config;
use omni_app_launcher::draw::draw_launcher_frame;
use omni_app_launcher::state::{self, OmniApp};
use omni_calculator::{draw_calculator, CalcState, CALC_HEIGHT, CALC_WIDTH};
use omni_daemon::Daemon;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: jemallocator::Jemalloc = jemallocator::Jemalloc;

enum ActiveApp {
    Launcher(Box<OmniApp>),
    Calculator(CalcState),
}

fn resize_for_app(daemon: &mut Daemon, app: &ActiveApp, config: &Config) {
    let (w, h) = match app {
        ActiveApp::Launcher(_) => {
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
        ActiveApp::Calculator(_) => (CALC_WIDTH, CALC_HEIGHT),
    };
    let (cw, ch) = daemon.surface_size();
    if w != cw || h != ch {
        daemon.resize_surface(w, h);
        daemon.poll();
    }
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
                    &config.font.family, config.theme.border_radius as f32, calc, cursor_visible,
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
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(tracing::level_filters::LevelFilter::INFO.into())
                .parse("info,omni_app_launcher=debug,omni=debug")
                .unwrap(),
        )
        .init();

    let config = Config::load();

    let mut daemon = Daemon::connect();
    daemon.init_globals();

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
    daemon.conn.flush().unwrap();

    daemon.poll();

    let mut active = ActiveApp::Launcher(Box::new(OmniApp::new(&config)));

    while daemon.running() {
        if !daemon.poll() { break; }

        let mut switch_to = None;

        match &mut active {
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
                            daemon.quit();
                        }
                        InputAction::Cancel => {
                            daemon.quit();
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
                            daemon.set_dirty();
                            switch_to = Some(ActiveApp::Launcher(Box::new(OmniApp::new(&config))));
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
        }

        if let Some(new_app) = switch_to {
            resize_for_app(&mut daemon, &new_app, &config);
            active = new_app;
        }

        let cursor_visible = daemon.cursor_visible();
        draw_frame(&mut daemon, &mut active, &config, cursor_visible);
        daemon.clear_dirty();
    }
}
