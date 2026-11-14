use std::error::Error;
use std::fs;
use std::path::Path;
use std::process::Command;

#[test]
fn generated_pal_tables_match_committed_csv() -> Result<(), Box<dyn Error>> {
    let scratch_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.agent/tmp");
    fs::create_dir_all(&scratch_root)?;
    let scratch_root = scratch_root.canonicalize()?;
    let scratch = scratch_root.join(format!("blep-csv-regression-{}", std::process::id()));
    fs::create_dir(&scratch)?;

    let output = Command::new(env!("CARGO_BIN_EXE_blep_generator"))
        .current_dir(&scratch)
        .output()?;
    assert!(
        output.status.success(),
        "Generator failed: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let fixtures = [
        (
            "blep_a500_filter_off.csv",
            include_str!("../output/blep_a500_filter_off.csv"),
        ),
        (
            "blep_a500_filter_on.csv",
            include_str!("../output/blep_a500_filter_on.csv"),
        ),
        (
            "blep_a1200_filter_off.csv",
            include_str!("../output/blep_a1200_filter_off.csv"),
        ),
        (
            "blep_a1200_filter_on.csv",
            include_str!("../output/blep_a1200_filter_on.csv"),
        ),
    ];
    for (filename, fixture) in fixtures {
        let generated =
            fs::read_to_string(scratch.join("tools/blep_generator/output").join(filename))?;
        let actual: Vec<_> = generated.lines().collect();
        let expected: Vec<_> = fixture.lines().collect();
        assert_eq!(
            actual.len(),
            expected.len(),
            "{filename}: row count changed"
        );
        for (row, (actual, expected)) in actual.iter().zip(&expected).enumerate() {
            assert_eq!(actual, expected, "{filename}: CSV row {row} changed");
        }
    }

    let generated = fs::read_to_string(scratch.join("crates/paula/src/audio_tables.rs"))?;
    assert_eq!(
        generated,
        include_str!("../../../crates/paula/src/audio_tables.rs").replace("\r\n", "\n"),
        "Runtime PAL/NTSC tables must match the generator"
    );
    fs::remove_dir_all(scratch)?;
    Ok(())
}
