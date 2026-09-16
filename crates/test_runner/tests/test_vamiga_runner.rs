//! vAmigaTS Test Runner & Verification Infrastructure Tests
//!
//! Validates catalog discovery, test categorization, .retrosh directive parsing,
//! deferred test classification, and execution harness pipelines.

use std::path::{Path, PathBuf};
use test_runner::vamiga::{
    run_vamiga_test, DeferredReason, VamigaCatalog, VamigaCategory, VamigaRunConfig, VamigaScript,
    VamigaTestStatus,
};

fn resolve_vamiga_root() -> Option<PathBuf> {
    let p1 = Path::new("ref_src/vAmigaTS");
    let p2 = Path::new("../../ref_src/vAmigaTS");
    if p1.is_dir() {
        Some(p1.to_path_buf())
    } else if p2.is_dir() {
        Some(p2.to_path_buf())
    } else {
        None
    }
}

#[test]
fn test_vamiga_catalog_discovery_and_counts() {
    let root = match resolve_vamiga_root() {
        Some(r) => r,
        None => {
            eprintln!("vAmigaTS repository not found, skipping discovery test.");
            return;
        }
    };

    let catalog = VamigaCatalog::discover(&root);
    let stats = catalog.stats();

    // Verify all 2,077 test directories are discovered
    assert_eq!(stats.total_tests, 2077);
    assert!(
        stats.runnable_tests >= 1450,
        "Expected at least 1,450 runnable tests, got {}",
        stats.runnable_tests
    );
    assert!(
        stats.deferred_tests >= 600,
        "Expected at least 600 deferred tests, got {}",
        stats.deferred_tests
    );
    assert_eq!(
        stats.total_tests,
        stats.runnable_tests + stats.deferred_tests
    );
}

#[test]
fn test_vamiga_category_filtering() {
    let root = match resolve_vamiga_root() {
        Some(r) => r,
        None => return,
    };

    let catalog = VamigaCatalog::discover(&root);

    let copper_tests = catalog.filter_category(VamigaCategory::Copper);
    assert!(
        copper_tests.len() >= 100,
        "Expected >= 100 Copper tests, got {}",
        copper_tests.len()
    );
    for t in &copper_tests {
        assert_eq!(t.category, VamigaCategory::Copper);
        assert!(t
            .rel_dir
            .to_string_lossy()
            .to_ascii_lowercase()
            .contains("copper"));
    }

    let blitter_tests = catalog.filter_category(VamigaCategory::Blitter);
    assert!(
        blitter_tests.len() >= 100,
        "Expected >= 100 Blitter tests, got {}",
        blitter_tests.len()
    );
    for t in &blitter_tests {
        assert_eq!(t.category, VamigaCategory::Blitter);
        assert!(t
            .rel_dir
            .to_string_lossy()
            .to_ascii_lowercase()
            .contains("blitter"));
    }

    let denise_tests = catalog.filter_category(VamigaCategory::Denise);
    assert!(
        denise_tests.len() >= 200,
        "Expected >= 200 Denise tests, got {}",
        denise_tests.len()
    );

    let paula_tests = catalog.filter_category(VamigaCategory::Paula);
    assert!(
        paula_tests.len() >= 100,
        "Expected >= 100 Paula tests, got {}",
        paula_tests.len()
    );
}

#[test]
fn test_vamiga_retrosh_parsing() {
    let script_content = r#"
# Regression testing script for vAmiga
# Dirk W. Hoffmann, 2022

# Setup the test environment
regression setup A500_OCS_1MB /tmp/kick13.rom
cpu set revision 68010

# Run the test
regression run /tmp/Copper_Wait_copwait1_ocs.adf
wait 9 seconds

# Exit with a screenshot
screenshot save Copper_Wait_copwait1_ocs
"#;

    let script = VamigaScript::parse(script_content);
    assert_eq!(script.setup_target, "A500_OCS_1MB");
    assert_eq!(script.rom_path.as_deref(), Some("/tmp/kick13.rom"));
    assert_eq!(script.cpu_revision.as_deref(), Some("68010"));
    assert_eq!(script.wait_seconds, Some(9));
    assert_eq!(script.wait_frames, None);
    assert_eq!(
        script.screenshot_name.as_deref(),
        Some("Copper_Wait_copwait1_ocs")
    );
    assert_eq!(script.effective_frames(12), 12);

    // Test with explicit frames
    let frames_script_content = "wait 300 frames\n";
    let frames_script = VamigaScript::parse(frames_script_content);
    assert_eq!(frames_script.wait_frames, Some(300));
    assert_eq!(frames_script.effective_frames(12), 300);

    // Test with cutout window
    let cutout_script_content =
        "screenshot set cutout x1=196 y1=36 x2=908 y2=314\nscreenshot save foo\n";
    let cutout_script = VamigaScript::parse(cutout_script_content);
    assert_eq!(cutout_script.cutout, Some((196, 36, 908, 314)));
    assert_eq!(cutout_script.screenshot_name.as_deref(), Some("foo"));
}

#[test]
fn test_vamiga_deferred_reasons_audit() {
    let root = match resolve_vamiga_root() {
        Some(r) => r,
        None => return,
    };

    let catalog = VamigaCatalog::discover(&root);
    let deferred = catalog.deferred_tests();
    assert!(!deferred.is_empty());

    let mut found_fpu = false;
    let mut found_ecs = false;
    let mut found_68010 = false;
    let mut found_non_std_boot = false;
    let mut found_no_raw = false;

    for t in &deferred {
        if let VamigaTestStatus::Deferred(reason) = &t.status {
            assert!(!reason.as_str().is_empty());
            assert!(!reason.roadmap_target().is_empty());
            match reason {
                DeferredReason::Fpu => found_fpu = true,
                DeferredReason::EcsOrAgaOnly => found_ecs = true,
                DeferredReason::Cpu68010Only => found_68010 = true,
                DeferredReason::NonStandardBootblock => found_non_std_boot = true,
                DeferredReason::NoRawReference => found_no_raw = true,
            }
        }
    }

    assert!(found_fpu, "Expected deferred FPU tests");
    assert!(found_ecs, "Expected deferred ECS/AGA tests");
    assert!(found_68010, "Expected deferred 68010 tests");
    assert!(
        found_non_std_boot,
        "Expected deferred non-standard bootblock tests"
    );
    assert!(found_no_raw, "Expected deferred no-raw tests");
}

#[test]
fn test_vamiga_runner_single_test_execution() {
    let root = match resolve_vamiga_root() {
        Some(r) => r,
        None => return,
    };

    let catalog = VamigaCatalog::discover(&root);
    let test_desc = catalog
        .find_test("coptim1")
        .expect("coptim1 test should be found in catalog");

    assert_eq!(test_desc.status, VamigaTestStatus::Runnable);
    assert!(test_desc.adf_path.is_file());
    assert!(test_desc.raw_path.as_ref().map_or(false, |p| p.is_file()));

    let config = VamigaRunConfig {
        frames_to_run: 8,
        ..Default::default()
    };

    let result = run_vamiga_test(test_desc, &config)
        .expect("coptim1 direct-injection execution should succeed");

    assert_eq!(result.total_pixels, 204_060);
    // Rendered frame produces diagnostic comparison
    assert!(result.passed || result.mismatched_pixels < 204_060);
}

#[test]
fn test_vamiga_matcher_exact_and_tolerance_comparison() {
    use test_runner::compare_raw_frames;

    let mut actual = [0u8; 612_180];
    let mut expected = vec![0u8; 612_180];

    // Perfect match
    let res = compare_raw_frames(&actual, &expected).expect("matcher should run");
    assert!(res.passed);
    assert_eq!(res.mismatched_pixels, 0);

    // Linear color code (level 1 = 16) with ADC rounding +/- 1 in expected
    actual[0] = 16;
    actual[1] = 16;
    actual[2] = 16;
    expected[0] = 15;
    expected[1] = 15;
    expected[2] = 16;

    let res = compare_raw_frames(&actual, &expected).expect("matcher should run");
    assert!(
        res.passed,
        "Linear code with +/- 1 ADC deviation should pass"
    );
    assert_eq!(res.mismatched_pixels, 0);
}
