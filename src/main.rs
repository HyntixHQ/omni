use wisp::draw::rgba_to_bgra;
use wisp::input::InputAction;

use omni_app_launcher::config::Config;
use omni_app_launcher::draw::draw_launcher_frame;
use omni_app_launcher::state::{self, OmniApp};
use omni_daemon::Daemon;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: jemallocator::Jemalloc = jemallocator::Jemalloc;

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

    // Snap body height to exact row multiple — no partial items, no gap
    let row_h = omni_app_launcher::draw::compute_row_height(
        daemon.font_system_mut(),
        config.font.size as f32,
        &config.font.family,
    );
    let visible_rows = 6.0f32;
    let padding = 10.0f32;
    let header_h = 42.0f32;
    let footer_h = 28.0f32;
    let gap = 8.0f32;
    let body_h = (visible_rows * row_h).round();
    let total_h = (padding + header_h + gap + body_h + gap + footer_h + padding).round() as i32;
    daemon.create_surface(config.window.width, total_h);
    daemon.conn.flush().unwrap();

    // Wait for configure
    daemon.poll();

    let mut app = OmniApp::new(&config);

    while daemon.running() {
        if !daemon.poll() { break; }

        // Resolve cursor from hovered region
        let pointer = daemon.pointer().cloned();
        let cursor = if daemon.mouse_inside() {
            app.mouse.cursor_at(daemon.mouse_y())
        } else {
            wisp::CursorStyle::Arrow
        };
        if let Some(ref p) = pointer {
            daemon.cursor().set_cursor(p, cursor);
        }

        // Scroll
        let row_height = omni_app_launcher::draw::compute_row_height(
            daemon.font_system_mut(),
            config.font.size as f32,
            &config.font.family,
        );
        let delta = daemon.drain_scroll(row_height);
        if delta != 0.0
            && app.mouse.region_at(daemon.mouse_y(), daemon.mouse_y()) == Some(wisp::events::RegionId::Body)
        {
            // Convert from old positive-scroll-down convention to GPUI negative convention
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
                    state::handle_action(&mut app, action, row_height, body_h);
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
            state::handle_action(&mut app, action, row_height, body_h);
            daemon.set_dirty();
            daemon.mark_input();
        }

        if daemon.is_dirty() {
            let cursor_visible = daemon.cursor_visible();
            let mouse_y = if daemon.mouse_inside() { daemon.mouse_y() } else { -9999.0 };
            daemon.draw_frame(|surf, font, swash| {
                draw_launcher_frame(
                    &mut surf.pixmap, font, swash, &mut app, &config, cursor_visible,
                    mouse_y,
                );
                if let Some(mmap) = surf.mmap.as_mut() {
                    rgba_to_bgra(surf.pixmap.data(), unsafe {
                        std::slice::from_raw_parts_mut(mmap.as_mut_ptr(), surf.pixmap.data().len())
                    });
                }
            });
            daemon.commit();
            daemon.clear_dirty();
        } else {
            // Always redraw for blinking cursor
            let cursor_visible = daemon.cursor_visible();
            let mouse_y = if daemon.mouse_inside() { daemon.mouse_y() } else { -9999.0 };
            daemon.draw_frame(|surf, font, swash| {
                draw_launcher_frame(
                    &mut surf.pixmap, font, swash, &mut app, &config, cursor_visible,
                    mouse_y,
                );
                if let Some(mmap) = surf.mmap.as_mut() {
                    rgba_to_bgra(surf.pixmap.data(), unsafe {
                        std::slice::from_raw_parts_mut(mmap.as_mut_ptr(), surf.pixmap.data().len())
                    });
                }
            });
            daemon.commit();
        }
    }
}
