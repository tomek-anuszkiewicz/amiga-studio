//! Keep brief departures from the foreground visible to the benchmark.

const FOCUS_LOST_ID: &str = "game-fps-focus-lost";

pub fn focused(ctx: &egui::Context) -> bool {
    let lost =
        ctx.data(|data| data.get_temp::<bool>(egui::Id::unique(FOCUS_LOST_ID)) == Some(true));
    #[cfg(not(target_arch = "wasm32"))]
    // Eframe reads the Winit window here. The input flag holds the last event delivered to egui.
    return !lost && ctx.input(|input| input.viewport().focused == Some(true));
    #[cfg(target_arch = "wasm32")]
    return !lost && crate::host_runtime::host_window_active();
}

pub fn watch_focus(ctx: &egui::Context) {
    ctx.data_mut(|data| data.insert_temp(egui::Id::unique(FOCUS_LOST_ID), false));
}

pub fn record_focus_loss(ctx: &egui::Context) {
    let listening = ctx.data_mut(|data| {
        let id = egui::Id::unique(FOCUS_LOST_ID);
        if data.get_temp::<bool>(id).is_some() {
            data.insert_temp(id, true);
            true
        } else {
            false
        }
    });
    if listening {
        ctx.request_repaint();
    }
}

pub fn stop_watching_focus(ctx: &egui::Context) {
    ctx.data_mut(|data| data.remove::<bool>(egui::Id::unique(FOCUS_LOST_ID)));
}
