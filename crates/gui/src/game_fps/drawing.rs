//! Observe the screen painter on retained egui passes and track foreground status.

use super::submissions::{Sample, Submissions};
use crate::host_video::FrameInfo;
use std::time::Duration;

const COUNTER_ID: &str = "game-fps-counter";
const DRAW_ID: &str = "game-fps-draw";
const FOCUS_LOST_ID: &str = "game-fps-focus-lost";

#[derive(Clone, Copy, Debug)]
struct Draw {
    info: FrameInfo,
    submitted: bool,
}

#[derive(Clone, Copy, Debug)]
struct Counter {
    #[cfg(not(target_arch = "wasm32"))]
    clock: std::time::Instant,
}

impl Counter {
    fn seconds(self) -> f64 {
        #[cfg(not(target_arch = "wasm32"))]
        return self.clock.elapsed().as_secs_f64();
        #[cfg(target_arch = "wasm32")]
        return crate::host_runtime::host_now();
    }
}

impl egui::Plugin for Counter {
    fn debug_name(&self) -> &'static str {
        "game-fps"
    }

    fn on_end_pass(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx();
        let retained = !ctx.will_discard();
        let wake = ctx.data_mut(|data| {
            let draw_id = egui::Id::unique(DRAW_ID);
            let draw = data.get_temp::<Draw>(draw_id);
            data.remove::<Draw>(draw_id);
            let counter_id = egui::Id::unique(COUNTER_ID);
            let mut counter = data.get_temp::<Submissions>(counter_id)?;
            let drawn = draw.filter(|draw| draw.submitted).map(|draw| draw.info);
            let wake = counter.observe(retained, drawn, self.seconds());
            data.insert_temp(counter_id, counter);
            wake
        });
        if let Some(seconds) = wake {
            ctx.request_repaint_after(Duration::from_secs_f64(seconds));
        }
    }
}

pub(super) fn install(ctx: &egui::Context) {
    ctx.add_plugin(Counter {
        #[cfg(not(target_arch = "wasm32"))]
        clock: std::time::Instant::now(),
    });
}

pub(super) fn focused(ctx: &egui::Context) -> bool {
    let lost =
        ctx.data(|data| data.get_temp::<bool>(egui::Id::unique(FOCUS_LOST_ID)) == Some(true));
    #[cfg(not(target_arch = "wasm32"))]
    // Eframe reads the Winit window here. The input flag holds the last event delivered to egui.
    return !lost && ctx.input(|input| input.viewport().focused == Some(true));
    #[cfg(target_arch = "wasm32")]
    return !lost && crate::host_runtime::host_window_active();
}

pub(super) fn watch_focus(ctx: &egui::Context) {
    ctx.data_mut(|data| data.insert_temp(egui::Id::unique(FOCUS_LOST_ID), false));
}

pub(crate) fn record_focus_loss(ctx: &egui::Context) {
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

pub(super) fn arm(ctx: &egui::Context, generation: u64, boot_field: u64) {
    ctx.data_mut(|data| {
        data.insert_temp(
            egui::Id::unique(COUNTER_ID),
            Submissions::new(generation, boot_field),
        );
    });
}

pub(crate) fn prepare_draw(ctx: &egui::Context, info: Option<FrameInfo>) {
    if let Some(info) = info {
        ctx.data_mut(|data| {
            data.insert_temp(
                egui::Id::unique(DRAW_ID),
                Draw {
                    info,
                    submitted: false,
                },
            );
        });
    }
}

pub(crate) fn record_draw(ctx: &egui::Context) {
    ctx.data_mut(|data| {
        if let Some(mut draw) = data.get_temp::<Draw>(egui::Id::unique(DRAW_ID)) {
            draw.submitted = true;
            data.insert_temp(egui::Id::unique(DRAW_ID), draw);
        }
    });
}

pub(super) fn sample(ctx: &egui::Context) -> Option<Sample> {
    ctx.data(|data| {
        data.get_temp::<Submissions>(egui::Id::unique(COUNTER_ID))
            .and_then(|counter| counter.sample)
    })
}

pub(super) fn disarm(ctx: &egui::Context) {
    ctx.data_mut(|data| {
        data.remove::<Submissions>(egui::Id::unique(COUNTER_ID));
        data.remove::<bool>(egui::Id::unique(FOCUS_LOST_ID));
    });
}
