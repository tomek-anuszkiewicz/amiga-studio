//! vAmigaTS RetroShell Script Parser
//!
//! Parses `.retrosh` test directives (`regression setup`, `wait`, `cpu set`, `screenshot save`)
//! to configure machine hardware parameters and frame execution budgets.

use std::fs;
use std::path::Path;

/// Parsed directives and configuration from a `.retrosh` script
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VamigaScript {
    /// Target machine setup profile (e.g. "A500_OCS_1MB", "A500_ECS_1MB")
    pub setup_target: String,
    /// Path to Kickstart ROM specified in script (if any)
    pub rom_path: Option<String>,
    /// CPU revision override (e.g. "68010")
    pub cpu_revision: Option<String>,
    /// Wait duration in seconds (if specified via `wait N seconds`)
    pub wait_seconds: Option<u32>,
    /// Wait duration in video frames (if specified via `wait N frames`)
    pub wait_frames: Option<u32>,
    /// Screenshot save target identifier
    pub screenshot_name: Option<String>,
    /// Cutout window `(x1, y1, x2, y2)` if configured via `screenshot set cutout`
    pub cutout: Option<(isize, isize, isize, isize)>,
}

impl Default for VamigaScript {
    fn default() -> Self {
        Self {
            setup_target: "A500_OCS_1MB".to_string(),
            rom_path: None,
            cpu_revision: None,
            wait_seconds: Some(9),
            wait_frames: None,
            screenshot_name: None,
            cutout: None,
        }
    }
}

impl VamigaScript {
    /// Parses script directives from text content
    pub fn parse(content: &str) -> Self {
        let mut script = Self::default();

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            let tokens: Vec<&str> = trimmed.split_whitespace().collect();
            if tokens.is_empty() {
                continue;
            }

            match tokens[0].to_ascii_lowercase().as_str() {
                "regression" if tokens.len() >= 3 && tokens[1].eq_ignore_ascii_case("setup") => {
                    script.setup_target = tokens[2].to_string();
                    if tokens.len() >= 4 {
                        script.rom_path = Some(tokens[3].to_string());
                    }
                }
                "wait" if tokens.len() >= 3 => {
                    if let Ok(val) = tokens[1].parse::<u32>() {
                        let unit = tokens[2].to_ascii_lowercase();
                        if unit.starts_with("second") || unit.starts_with("sec") {
                            script.wait_seconds = Some(val);
                        } else if unit.starts_with("frame") {
                            script.wait_frames = Some(val);
                        }
                    }
                }
                "cpu" if tokens.len() >= 4 && tokens[1].eq_ignore_ascii_case("set") => {
                    if tokens[2].eq_ignore_ascii_case("revision") {
                        script.cpu_revision = Some(tokens[3].to_string());
                    }
                }
                "screenshot" if tokens.len() >= 3 && tokens[1].eq_ignore_ascii_case("save") => {
                    script.screenshot_name = Some(tokens[2].to_string());
                }
                "screenshot"
                    if tokens.len() >= 4
                        && tokens[1].eq_ignore_ascii_case("set")
                        && tokens[2].eq_ignore_ascii_case("cutout") =>
                {
                    let mut x1: Option<isize> = None;
                    let mut y1: Option<isize> = None;
                    let mut x2: Option<isize> = None;
                    let mut y2: Option<isize> = None;
                    for token in &tokens[3..] {
                        if let Some((k, v)) = token.split_once('=') {
                            let val = v.parse::<isize>().ok();
                            match k.to_ascii_lowercase().as_str() {
                                "x1" => x1 = val,
                                "y1" => y1 = val,
                                "x2" => x2 = val,
                                "y2" => y2 = val,
                                _ => {}
                            }
                        }
                    }
                    if let (Some(x1), Some(y1), Some(x2), Some(y2)) = (x1, y1, x2, y2) {
                        script.cutout = Some((x1, y1, x2, y2));
                    }
                }
                _ => {}
            }
        }

        script
    }

    /// Loads and parses a `.retrosh` script from disk
    pub fn load_from_file(path: &Path) -> Result<Self, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read script {:?}: {}", path, e))?;
        Ok(Self::parse(&content))
    }

    /// Computes the effective frame execution budget for direct-injection execution
    ///
    /// In full floppy boot, Kickstart consumes ~6-7 seconds before payload execution.
    /// In direct injection, the test starts executing immediately at frame 0.
    /// Thus a test specifying `wait 9 seconds` runs cleanly within `default_direct_frames` (default: 8..16 frames).
    pub fn effective_frames(&self, default_direct_frames: u32) -> u32 {
        if let Some(frames) = self.wait_frames {
            frames.max(1)
        } else {
            default_direct_frames.max(1)
        }
    }
}
