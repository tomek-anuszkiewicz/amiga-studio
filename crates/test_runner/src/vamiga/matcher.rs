//! vAmigaTS Golden Raw Reference Frame Differencer
//!
//! Compares 716 x 285 24-bit RGB frame buffers against verified vAmigaTS `.raw` reference captures (612,180 bytes).

/// Width of the standard vAmigaTS reference image in pixels
pub const VAMIGA_RAW_WIDTH: usize = 716;

/// Height of the standard vAmigaTS reference image in scanlines
pub const VAMIGA_RAW_HEIGHT: usize = 285;

/// Total pixel count of the vAmigaTS reference viewport
pub const VAMIGA_RAW_PIXELS: usize = VAMIGA_RAW_WIDTH * VAMIGA_RAW_HEIGHT;

/// Total size in bytes of the uncompressed 24-bit RGB reference image (716 * 285 * 3)
pub const VAMIGA_RAW_BYTE_SIZE: usize = VAMIGA_RAW_PIXELS * 3;

/// Tolerance per color channel (+/- 1) to account for YUV chroma subcarrier rounding and quantization
pub const COLOR_TOLERANCE_PER_CHANNEL: i16 = 1;

/// Details of an individual pixel mismatch between rendered and reference frames
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VamigaDiff {
    /// Horizontal pixel coordinate (0..715) within the viewport
    pub x: usize,
    /// Vertical scanline coordinate (0..284) within the viewport
    pub y: usize,
    /// Rendered RGB value [R, G, B]
    pub actual: [u8; 3],
    /// Expected reference RGB value [R, G, B]
    pub expected: [u8; 3],
}

/// Comprehensive outcome of comparing rendered video against reference `.raw` frame
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VamigaTestResult {
    /// True if all 204,060 pixels match reference exactly
    pub passed: bool,
    /// Total count of pixels that diverge from reference
    pub mismatched_pixels: usize,
    /// Total pixel count in frame (204,060)
    pub total_pixels: usize,
    /// Details of the first mismatched pixel encountered (if any)
    pub first_mismatch: Option<VamigaDiff>,
}

/// Compares a 612,180-byte rendered RGB frame against reference bytes.
pub fn compare_raw_frames(
    actual: &[u8; VAMIGA_RAW_BYTE_SIZE],
    expected: &[u8],
) -> Result<VamigaTestResult, String> {
    if expected.len() != VAMIGA_RAW_BYTE_SIZE {
        return Err(format!(
            "Reference file size mismatch: expected {} bytes, got {}",
            VAMIGA_RAW_BYTE_SIZE,
            expected.len()
        ));
    }

    let mut mismatched_pixels = 0usize;
    let mut first_mismatch = None;

    for pixel_idx in 0..VAMIGA_RAW_PIXELS {
        let byte_offset = pixel_idx * 3;
        let act_r = actual[byte_offset];
        let act_g = actual[byte_offset + 1];
        let act_b = actual[byte_offset + 2];

        let exp_r = expected[byte_offset];
        let exp_g = expected[byte_offset + 1];
        let exp_b = expected[byte_offset + 2];

        let diff_r = (act_r as i16 - exp_r as i16).abs();
        let diff_g = (act_g as i16 - exp_g as i16).abs();
        let diff_b = (act_b as i16 - exp_b as i16).abs();

        if diff_r > COLOR_TOLERANCE_PER_CHANNEL
            || diff_g > COLOR_TOLERANCE_PER_CHANNEL
            || diff_b > COLOR_TOLERANCE_PER_CHANNEL
        {
            mismatched_pixels += 1;
            if first_mismatch.is_none() {
                let x = pixel_idx % VAMIGA_RAW_WIDTH;
                let y = pixel_idx / VAMIGA_RAW_WIDTH;
                first_mismatch = Some(VamigaDiff {
                    x,
                    y,
                    actual: [act_r, act_g, act_b],
                    expected: [exp_r, exp_g, exp_b],
                });
            }
        }
    }

    let passed = mismatched_pixels == 0;
    Ok(VamigaTestResult {
        passed,
        mismatched_pixels,
        total_pixels: VAMIGA_RAW_PIXELS,
        first_mismatch,
    })
}
