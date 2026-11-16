#![allow(clippy::unwrap_used)]
mod common;
use gui::host_commands::HostCommand;
use gui::{EmulatorApp, ViewMode};

fn running() -> (EmulatorApp, egui::Context) {
    let mut session = Box::new(gui::DebuggerSession::new());
    session.load_binary(0x1000, &[0x60, 0xfe], true).unwrap();
    let mut app = EmulatorApp::with_session(session);
    app.set_view_mode(ViewMode::ScreenOnly);
    let ctx = egui::Context::default();
    common::settle(&mut app, &ctx);
    common::command(&mut app, HostCommand::Run);
    app.player.active_configuration = Some(Default::default());
    ctx.memory_mut(|memory| memory.request_focus(egui::Id::new("amiga-screen-input")));
    app.player.captured = true;
    draw(&mut app, &ctx, None);
    (app, ctx)
}
fn draw(app: &mut EmulatorApp, ctx: &egui::Context, key: Option<egui::Key>) {
    let events = key
        .map(|key| egui::Event::Key {
            key,
            physical_key: Some(key),
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        })
        .into_iter()
        .collect();
    let _ = ctx.run(
        egui::RawInput {
            focused: true,
            events,
            ..Default::default()
        },
        |ctx| app.update_ui(ctx),
    );
}
#[test]
fn captured_tab_keeps_guest_focus_instead_of_navigating_host_controls() {
    let (mut app, ctx) = running();
    assert!(app.player.captured);
    draw(&mut app, &ctx, Some(egui::Key::Tab));
    assert!(ctx.memory(|memory| memory.has_focus(egui::Id::new("amiga-screen-input"))));
    assert!(app.player.captured);
}
#[test]
fn enter_bound_to_release_does_not_recapture_through_a_keyboard_canvas_click() {
    let (mut app, ctx) = running();
    app.player
        .active_configuration
        .as_mut()
        .unwrap()
        .frontend
        .release = "Enter".into();
    draw(&mut app, &ctx, Some(egui::Key::Enter));
    assert!(!app.player.captured);
}

#[test]
fn long_disk_names_keep_the_wide_player_panel_compact() {
    use egui_kittest::{kittest::Queryable, Harness};
    let mut app = EmulatorApp::default();
    app.set_view_mode(ViewMode::ScreenOnly);
    app.player.disks.push(gui::player::SessionDisk {
        id: "long".into(),
        label:
            "Workbench v1.3 rev 34.20 (1988)(Commodore)(A500-A2000)(Disk 1 of 2)(Workbench)[m].adf"
                .into(),
        bytes: None,
    });
    app.player.mounted[0] = Some("long".into());
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .build_state(|ctx, app: &mut EmulatorApp| app.update_ui(ctx), app);
    common::settle_harness(&mut harness);
    let save = harness.get_by_label("Save").raw_bounds().unwrap();
    assert!(
        save.x0 >= 1000.0 && save.x1 <= 1280.0,
        "Long media label stretched the player panel: {save:?}"
    );
}
