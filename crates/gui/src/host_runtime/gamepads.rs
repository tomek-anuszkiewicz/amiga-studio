//! Native controller collection; keyboard and mouse use window events.

use super::NATIVE_GAMEPADS;

pub fn collect_native_gamepad(
    gamepads: &mut crate::host_gamepads::NativeGamepads,
    observed: std::time::SystemTime,
) {
    let Ok(target) = NATIVE_GAMEPADS.lock() else {
        return;
    };
    let Some(sender) = &*target else {
        return;
    };
    let Ok(mut mailboxes) = sender.shared.mailboxes.lock() else {
        return;
    };
    let fresh = observed >= mailboxes.gamepads_started_at;
    let active = mailboxes.gamepads_active() && fresh;
    if mailboxes.sample_gamepads(gamepads.samples(), active, fresh) {
        sender.repaint.request_repaint();
    }
}
