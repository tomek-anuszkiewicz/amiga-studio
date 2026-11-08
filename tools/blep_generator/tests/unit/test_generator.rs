use crate::config::{AmigaAudioFilterParams, LedFilter, TvMode};
use crate::generator::{generate_blep, sinc, BLEP_TABLE_LENGTH};

#[test]
fn test_sinc_zero() {
    assert_eq!(sinc(0.0), 1.0);
}

#[test]
fn test_blep_generation_bounds() {
    let a500_params = AmigaAudioFilterParams::a500();
    let blep = generate_blep(&a500_params, TvMode::Pal, LedFilter::Off);
    assert_eq!(blep.len(), BLEP_TABLE_LENGTH);
    // First elements should be around 131072 (full scale)
    assert!((blep[0] - 131072).abs() < 10);
    // Last elements should settle near 0
    assert!(blep[blep.len() - 1].abs() < 10);
}
