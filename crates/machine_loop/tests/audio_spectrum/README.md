# Audio reconstruction spectrum test

Run from the repository root:

```powershell
cargo test -p machine_loop --test test_audio_spectrum -- --nocapture
```

The [integration test](../test_audio_spectrum.rs) writes signed eight-bit periodic
waveforms into Chip RAM, programs AUD0/1 and enables DMA. Agnus fetches the words;
Paula plays them and reconstructs 44.1 kHz stereo PCM. The host test drains the
ring after each video field, saves WAV files, reads them back and analyzes the
decoded samples. It never injects AUDxDAT samples or consults BLEP tables for its
expected output. The CPU remains stopped; this tests the programmed audio path,
not a guest application's initialization code.

Outputs are disposable and overwritten by the next run:

- `.agent/tmp/audio-spectrum/{pal,ntsc}-filter-{off,on}.wav`: stereo PCM16.
- `.agent/tmp/audio-spectrum/report.json`: expected and measured amplitudes,
  local spectral peaks, residuals, aliases and foreign-side tones.

## Stimulus and independent expectations

Each DMA block contains 256 bytes. The period is 128 CCK and volume is 64.
The right side contains harmonics 5, 37 and 83 of the block repetition rate;
the left contains harmonic 11. PAL frequencies are approximately 541, 4005 and
8984 Hz right, and 1191 Hz left. NTSC frequencies follow its different CCK rate.
CIA-A PA1 selects the LED filter through real bus writes.

After 4410 settling samples, a 16384-sample window provides about 0.372 seconds
of stationary output. Fourier projections use a periodic Hann window and divide
by its coherent gain. This prevents an arbitrary capture boundary from leaking
large tone energy across the spectrum. The method follows the amplitude-scaling
and windowing principles in the [SciPy spectral analysis guide](https://docs.scipy.org/doc/scipy/tutorial/signal.html#spectral-analysis)
and [Hann window documentation](https://docs.scipy.org/doc/scipy/reference/generated/scipy.signal.windows.hann.html).

The expected amplitude starts with a full-period DFT of the actual quantized
input bytes, then applies the DAC hold sinc, the 4.9 kHz RC circuit and optional
3.275 kHz, Q=0.65 LED filter from the [filter model](../../../../tools/blep_generator/README.md).
A direct frequency-domain sum evaluates the specified 2048-point, beta-8 Kaiser
sinc with a 21 kHz cutoff. Its Bessel function uses a power series, independently
of the generator's Chebyshev approximation. The oracle neither generates a BLEP
table nor performs minimum-phase conversion or step-history reconstruction.

DAC hold also produces spectral images. The high right-side tone has a lower
image near 18.7 kHz in PAL, inside the output Nyquist band. The test checks that
image's frequency and filtered amplitude too. It subtracts these expected
components before measuring unexplained residual power. Images above Nyquist
must be suppressed to avoid folding into false audible tones.

## Acceptance and limits

- Every expected tone and in-band image: amplitude error below 0.5 dB.
- Local peak: within one quarter of a window bin, about 0.673 Hz.
- Residual RMS after fitting expected components: below -35 dB relative to PCM.
- Folded first upper DAC image: below -45 dBc relative to its source tone.
- Foreign-side tone: below -42 dBc, allowing eight-bit quantization harmonics.
- Every generated PCM pair reaches the WAV in order; no ring samples are lost.

The amplitude tolerance allows rounded circuit parameters and finite filter
reconstruction. Residual and foreign-tone limits allow source quantization.
Four negative controls corrupt the captured PAL samples: halve gain, swap sides,
drop one 882-pair block, and add an alias at two percent of its source tone.
The spectral checks must reject all four.

This validates the implemented signal path against a mathematical filter model.
It does not establish physical Amiga component tolerances, exact hardware phase,
filter-switching transients, host playback, or every possible DMA workload.

## DMA attachment output tests

The [attachment tests](attachment.rs) also run with the command above. To select
only these two scenarios:

```powershell
cargo test -p machine_loop --test test_audio_spectrum attachment:: -- --nocapture
```

Channel 0 reads a looping block of big-endian control words from Chip RAM.
Channel 1 reads a quantized sine waveform. Both use real DMA; the host makes no
parameter writes after starting playback. The two tests exercise:

- Volume attachment: 16 -> 48 -> 16, with amplitude ratio 3 within 1% and return
  to the initial amplitude within 1%.
- Period attachment: 256 -> 512 -> 256 CCK, with frequency ratio 2 within 0.2%
  and return to the initial frequency within 0.2%.

Each control plateau contains 64 words at a 4096-CCK source period. Three
plateaus cover both transition directions and the control block's DMA restart.
The 4096-sample analysis windows begin one sixth of a plateau after its expected
start, allowing DMA startup, destination latching and filter settling. Their
positions follow the known stimulus clock rather than observed channel state.
RMS measures volume, interpolated positive zero crossings measure frequency,
and the existing Hann Fourier projection checks that the expected sine accounts
for RMS within 2%. Each plateau's frequency must also match its configured
period and video standard within 0.2%.

PAL and NTSC run with the LED filter off. Every captured right-side sample must
be exactly zero: channel 0 supplies modulation data and must never be audible,
including during transitions. Frame drains retain every generated PCM pair.
The tests write and read back
`.agent/tmp/audio-spectrum/{pal,ntsc}-{volume,period}-attachment.wav` before
analyzing the decoded left channel. Measurements are printed with `--nocapture`.
These tests check settled modulation levels. Channel-state tests separately
check when attachment parameters are transferred.
