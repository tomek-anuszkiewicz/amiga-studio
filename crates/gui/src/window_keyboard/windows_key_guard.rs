//! Foreground Windows logo key suppression for the emulator window.

use std::num::NonZeroIsize;
use windows::core::{IUnknown, Interface};
use windows::Win32::Devices::HumanInterfaceDevice::{
    DirectInput8Create, GUID_SysKeyboard, IDirectInput8W, IDirectInputDevice8W, DIDATAFORMAT,
    DISCL_FOREGROUND, DISCL_NONEXCLUSIVE, DISCL_NOWINKEY,
};
use windows::Win32::Foundation::{HINSTANCE, HWND};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;

#[allow(unsafe_code)]
#[link(name = "dinput8", kind = "static", modifiers = "-bundle")]
#[link(name = "dxguid", kind = "static", modifiers = "-bundle")]
unsafe extern "system" {
    static c_dfDIKeyboard: DIDATAFORMAT;
}

#[derive(Default)]
pub(super) struct WindowsKeyGuard {
    requested: bool,
    device: Option<IDirectInputDevice8W>,
}

impl WindowsKeyGuard {
    pub(super) fn update(&mut self, window: Option<NonZeroIsize>, active: bool) {
        if !active {
            self.release();
        } else if let Some(window) = window.filter(|_| !self.requested) {
            self.requested = true;
            match acquire(window) {
                Ok(device) => self.device = Some(device),
                Err(error) => eprintln!("Could not suppress Windows logo keys: {error}"),
            }
        }
    }

    #[allow(unsafe_code)]
    pub(super) fn release(&mut self) {
        self.requested = false;
        if let Some(device) = self.device.take() {
            // The device is owned by this event loop and was acquired here.
            // Foreground loss may already have unacquired it.
            let _ = unsafe { device.Unacquire() };
        }
    }
}

impl Drop for WindowsKeyGuard {
    fn drop(&mut self) {
        self.release();
    }
}

#[allow(unsafe_code)]
fn acquire(window: NonZeroIsize) -> windows::core::Result<IDirectInputDevice8W> {
    // The event loop supplies the live top-level window and keeps the device
    // on that thread. The SDK data format is static and read-only.
    unsafe {
        let module = GetModuleHandleW(None)?;
        let mut interface = std::ptr::null_mut();
        DirectInput8Create(
            HINSTANCE(module.0),
            0x0800,
            &IDirectInput8W::IID,
            &mut interface,
            None::<&IUnknown>,
        )?;
        // DirectInput returned an owned reference for the requested IID.
        let input = IDirectInput8W::from_raw(interface);
        let mut device = None;
        input.CreateDevice(&GUID_SysKeyboard, &mut device, None::<&IUnknown>)?;
        let device = device.ok_or_else(windows::core::Error::empty)?;
        device.SetDataFormat((&raw const c_dfDIKeyboard).cast_mut())?;
        device.SetCooperativeLevel(
            HWND(window.get() as *mut std::ffi::c_void),
            DISCL_FOREGROUND | DISCL_NONEXCLUSIVE | DISCL_NOWINKEY,
        )?;
        device.Acquire()?;
        Ok(device)
    }
}
