#![cfg(not(target_arch = "wasm32"))]
#![allow(clippy::unwrap_used, clippy::panic)]

mod common;
use egui::{Context, RawInput, ViewportCommand, ViewportEvent, ViewportId};
use gui::EmulatorApp;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

#[derive(Default)]
struct Storage(HashMap<String, String>);

impl eframe::Storage for Storage {
    fn get_string(&self, key: &str) -> Option<String> {
        self.0.get(key).cloned()
    }
    fn set_string(&mut self, key: &str, value: String) {
        self.0.insert(key.to_owned(), value);
    }
    fn flush(&mut self) {}
}

#[test]
fn closing_waits_for_owner_shutdown_and_persists_its_final_battery() {
    let mut session = Box::new(gui::DebuggerSession::new());
    session.machine.rtc.load_battery(rtc::RtcBattery {
        offset_seconds: -777,
    });
    let mut app = EmulatorApp::with_session(session);
    let context = Context::default();
    common::settle(&mut app, &context);
    let mut input = RawInput::default();
    input
        .viewports
        .get_mut(&ViewportId::ROOT)
        .unwrap()
        .events
        .push(ViewportEvent::Close);
    let requested = context.run(input, |context| app.update_ui(context));
    assert!(requested.viewport_output[&ViewportId::ROOT]
        .commands
        .contains(&ViewportCommand::CancelClose));
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let output = context.run(RawInput::default(), |context| app.update_ui(context));
        if output.viewport_output[&ViewportId::ROOT]
            .commands
            .contains(&ViewportCommand::Close)
        {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "Close did not receive a shutdown acknowledgement"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(app.host.status.shutdown);
    assert_eq!(app.host.status.battery.offset_seconds, -777);
    let mut storage = Storage::default();
    eframe::App::save(&mut app, &mut storage);
    let battery: rtc::RtcBattery = eframe::get_value(&storage, "rtc_battery").unwrap();
    assert_eq!(battery.offset_seconds, -777);
}

#[test]
fn worker_notifications_deliver_initial_video_and_inspection_without_gui_polling() {
    static WAKE: AtomicBool = AtomicBool::new(false);
    for _ in 0..8 {
        let context = Context::default();
        context.set_request_repaint_callback(|_| {
            WAKE.store(true, Ordering::Release);
        });
        let mut app = EmulatorApp::default();
        WAKE.store(true, Ordering::Release);
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if WAKE.swap(false, Ordering::AcqRel) {
                let _ = context.run(RawInput::default(), |context| app.update_ui(context));
            }
            if app.host.is_idle() && app.host.presented.is_some() && app.host.data.is_some() {
                break;
            }
            assert!(Instant::now() < deadline, "An owner notification was lost");
            std::thread::sleep(Duration::from_millis(1));
        }
    }
}
