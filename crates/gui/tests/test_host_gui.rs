#![cfg(not(target_arch = "wasm32"))]
#![allow(clippy::unwrap_used, clippy::panic)]

mod common;
use egui::{Context, RawInput, ViewportCommand, ViewportEvent, ViewportId};
use gui::EmulatorApp;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

