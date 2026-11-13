//! Run the same standalone guest as the GUI, with deterministic host input.

#[path = "../tests/programs/display/mixed_scene/host.rs"]
mod scene;

use std::io::{BufWriter, Write};
use std::path::Path;
use std::time::Instant;

use config::VideoStandard;
use machine_loop::A500Machine;
use serde_json::json;

fn image(machine: &A500Machine, path: &Path) -> std::io::Result<()> {
    let frame = &machine.denise.frame_builder;
    let mut out = BufWriter::new(std::fs::File::create(path)?);
    write!(out, "P6\n{} {}\n255\n", frame.width, frame.height)?;
    for pixel in frame.frame_buffer() {
        out.write_all(&[(pixel >> 16) as u8, (pixel >> 8) as u8, *pixel as u8])?;
    }
    out.flush()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 1 || args.iter().any(|arg| arg == "--help") {
        println!(
            "Usage: mixed_scene audit|bench [--fields N] [--warmup N] [--series N] [--ntsc] [--output DIRECTORY]\n\
             Defaults: 32 warmup fields, audit 32 fields or bench 120 fields x 3 series.\n\
             Output: JSON report and final PPM image, outside the timed interval.\n\
             DMA counts are selected owner CCKs; CPU ownership includes unclaimed slots.\n\
             Benchmark includes deterministic input and audio draining, excludes GUI/pacing."
        );
        return Ok(());
    }
    let audit = match args[1].as_str() {
        "audit" => true,
        "bench" => false,
        _ => return Err("Expected audit or bench; use --help".into()),
    };
    let mut fields = if audit { 32u64 } else { 120 };
    let mut warmup = scene::WARMUP_FIELDS;
    let mut series = if audit { 1u64 } else { 3 };
    let mut standard = VideoStandard::Pal;
    let mut output = None;
    let mut index = 2;
    while index < args.len() {
        match args[index].as_str() {
            "--ntsc" => standard = VideoStandard::Ntsc,
            option @ ("--fields" | "--warmup" | "--series" | "--output") => {
                index += 1;
                let value = args.get(index).ok_or("Missing option value")?;
                match option {
                    "--fields" => fields = value.parse()?,
                    "--warmup" => warmup = value.parse()?,
                    "--series" => series = value.parse()?,
                    _ => output = Some(std::path::PathBuf::from(value)),
                }
            }
            _ => return Err("Unknown option; use --help".into()),
        }
        index += 1;
    }
    if fields == 0 || series == 0 || (audit && series != 1) {
        return Err("Fields and series must be positive; audit uses one series".into());
    }
    let mut machine = scene::machine(standard);
    for field in 0..warmup {
        scene::run_field(&mut machine, field);
    }
    let before = scene::progress(&machine);
    let initial = machine.save_state();
    let mut timings = Vec::new();
    let mut activity = scene::Activity::default();
    let mut previous_result = None;
    for run in 0..series {
        // Every series executes the same guest work and external event schedule.
        machine.load_state(&initial);
        if audit {
            for field in warmup..warmup + fields {
                scene::audit_field(&mut machine, field, &mut activity);
            }
        } else {
            let start = Instant::now();
            for field in warmup..warmup + fields {
                scene::run_field(&mut machine, field);
            }
            let seconds = start.elapsed().as_secs_f64();
            println!(
                "series={} fields={} seconds={seconds:.6} ms_per_field={:.6} fields_per_second={:.3}",
                run + 1, fields, seconds * 1000.0 / fields as f64, fields as f64 / seconds
            );
            timings.push(seconds);
        }
        let result = machine.save_state();
        if previous_result
            .as_ref()
            .is_some_and(|previous| previous != &result)
        {
            return Err("Repeated series ended in different machine states".into());
        }
        previous_result = Some(result);
    }
    let after = scene::progress(&machine);
    let report = json!({
        "mode": if audit { "audit" } else { "bench" },
        "standard": format!("{standard:?}"),
        "warmup_fields": warmup,
        "fields_per_series": fields,
        "series_seconds": timings,
        "emulated_ccks_per_series": machine.cck - initial.cck,
        "guest_before": before,
        "guest_after": after,
        "activity": if audit { Some(&activity) } else { None },
        "width": machine.denise.frame_builder.width,
        "height": machine.denise.frame_builder.height,
        "pc": machine.cpu.state.instruction_pc(),
        "halted": machine.cpu.state.is_halted(),
        "stopped": machine.cpu.state.is_stopped(),
    });
    println!("{}", serde_json::to_string_pretty(&report)?);
    if let Some(directory) = output {
        std::fs::create_dir_all(&directory)?;
        std::fs::write(
            directory.join("report.json"),
            serde_json::to_vec_pretty(&report)?,
        )?;
        image(&machine, &directory.join("frame.ppm"))?;
    }
    if after.fault != 0 || after.work_loops <= before.work_loops || after.fields <= before.fields {
        return Err("Guest made no progress or entered its exception handler".into());
    }
    Ok(())
}
