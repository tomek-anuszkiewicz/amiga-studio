# Amiga Paula Audio BLEP Table Generator

This tool generates Band-Limited Step (BLEP) interpolation tables for cycle-accurate anti-aliased sound synthesis in Paula's variable-rate audio channels.

## 1. Why Band-Limited Steps (BLEPs)?

Commodore Amiga's Paula chip features 4 independent DMA audio channels with variable sample playback rates (up to ~28 kHz on standard PAL screens).
In an emulator, changing sample levels on arbitrary clock cycles produces harsh high-frequency harmonics (Nyquist aliasing). Rather than brute-force oversampling the entire sound mixer at 3.54 MHz, BLEP synthesis injects pre-calculated bandlimited step responses whenever a DAC transition occurs, cancelling alias frequencies with minimal CPU overhead.

---

## 2. Mathematical Synthesis Pipeline

The generator models the exact analog filtering path of the Amiga 500 and Amiga 1200 hardware through a 6-stage pipeline:

```
┌─────────────────┐     ┌───────────────┐     ┌───────────────────────┐
│ Bandlimited     │ ──> │ Kaiser Window │ ──> │ Minimum-Phase         │
│ Sinc Impulse    │     │ (Bessel I₀)   │     │ (Real Cepstrum FFT)   │
└─────────────────┘     └───────────────┘     └───────────────────────┘
                                                          │
┌─────────────────┐     ┌───────────────┐                 ▼
│ Quantization &  │ <── │ Numerical     │ <── ┌───────────────────────┐
│ Scaling (2¹⁷)   │     │ Integration   │     │ Analog Circuit IIR    │
└─────────────────┘     └───────────────┘     │ (RC & Sallen-Key LED) │
                                              └───────────────────────┘
```

1. **Bandlimited Sinc Impulse**: Sinc kernel with $f_c = 21\,\text{kHz}$ at DAC clock $F_s \approx 3.546895\,\text{MHz}$ (PAL).
2. **Kaiser Windowing**: Windows the 2048-sample kernel using modified Bessel function of the first kind $I_0(x)$ ($\beta = 8.0$ for A500, $\beta = 9.0$ for A1200).
3. **Minimum-Phase Reconstruction (Physical Causality & Pre-Ringing Removal)**:
   - **Why it's needed (Causality)**: A standard mathematical sinc filter is symmetric (*linear phase*), meaning its impulse response starts oscillating **before** the event occurs (*pre-ringing / anticausal components*). In real physical audio hardware, an effect cannot precede its cause — sound can only react **after** (in the future of) a DAC step transition, never before it in the past.
   - **How it works**: By converting the filter to **minimum phase** via the real cepstrum, all energy is shifted to $t \ge 0$. Anticausal components (pre-ringing) are completely eliminated so that the BLEP step response begins strictly at and after the exact moment the DAC level switches:
     $$\text{cepstrum} = \text{IFFT}(\ln |\text{FFT}(h)|)$$
   - Anticausal components ($t < 0$) are zeroed, and causal components are doubled before exponential inversion:
     $$h_{\text{min}} = \text{IFFT}(\exp(\text{FFT}(\text{cepstrum})))$$
4. **Analog Circuit Emulation (IIR Biquad Filters)**:
   - Discretized via the Bilinear Transform ($s \to z$ domain).
   - **Fixed Low-Pass**: 1st-order RC filter ($R_{331} / C_{331}$, $f_c \approx 4.9\,\text{kHz}$ on A500, $f_c \approx 32\,\text{kHz}$ on A1200).
   - **Switchable LED Filter**: 2nd-order Sallen-Key low-pass filter ($R_{332}, R_{333}, C_{332}, C_{333}$, $f_0 \approx 3275\,\text{Hz}$, $Q \approx 0.65$).
5. **Numerical Step Integration**: Integrates the impulse response into a step response:
   $$s[n] = -\sum_{k} h[k] + \sum_{k=0}^n h[k]$$
6. **Quantization & Scaling**: Normalizes and scales to 18-bit fixed point dynamic range ($2^{17} = 131072$).

---

## 3. Output Files & WinUAE Comparison

Running `cargo run -p blep_generator` generates 4 CSV files and a ready-to-compile Rust module in `tools/blep_generator/output/`:

| Hardware Target | Filter Switch | Generated CSV File | Max Error vs WinUAE | Relative Error (% FS) |
|---|---|---|---|---|
| **Amiga 500** | Filter OFF | `blep_a500_filter_off.csv` | **109** / 131072 | **0.0832%** |
| **Amiga 500** | Filter ON  | `blep_a500_filter_on.csv`  | **42** / 131072  | **0.0320%** |
| **Amiga 1200**| Filter OFF | `blep_a1200_filter_off.csv`| **342** / 131072 | **0.2609%** |
| **Amiga 1200**| Filter ON  | `blep_a1200_filter_on.csv` | **97** / 131072  | **0.0740%** |

### Rust Source Output for Paula Core
- **`tools/blep_generator/output/blep_tables.rs`**: Exports `BLEP_A500_FILTER_OFF`, `BLEP_A500_FILTER_ON`, `BLEP_A1200_FILTER_OFF`, `BLEP_A1200_FILTER_ON` as static const arrays (`[i32; 2048]`), ready for zero-allocation static embedding directly into Paula's audio rendering engine.

*All 4 curves match WinUAE ground truth within < 0.27% across the entire dynamic range.*
A visual vector plot comparing the curves and residuals is exported to `tools/blep_generator/output/blep_comparison.svg`.
