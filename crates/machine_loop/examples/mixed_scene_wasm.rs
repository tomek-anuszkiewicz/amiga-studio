//! Browser-worker entry point for the uncapped Mixed Scene benchmark.

#[cfg(target_arch = "wasm32")]
#[path = "../../../tests/mixed_scene/host.rs"]
mod host;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = performance, js_name = now)]
    fn performance_now() -> f64;
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn run_mixed_scene() -> Result<String, JsValue> {
    let standard = config::VideoStandard::Pal;
    let mut machine = host::machine(standard);
    host::benchmark(
        &mut machine,
        standard,
        host::WARMUP_FIELDS,
        host::BENCH_FIELDS,
        host::BENCH_SERIES,
        || performance_now() / 1000.0,
    )
    .map(|report| report.to_string())
    .map_err(JsValue::from_str)
}

fn main() {}
