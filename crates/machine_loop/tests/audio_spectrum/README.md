# Audio reconstruction spectrum test

Run the test from the repository root.

```powershell
python tools/verification/run_rust_tests.py -- cargo test -p machine_loop --test test_audio_spectrum -- --nocapture
```

The [integration test](../test_audio_spectrum.rs) writes signed eight-bit periodic
waveforms into Chip RAM, programs AUD0/1, and enables DMA. Agnus fetches the words
and passes them to Paula, which plays them and reconstructs 44.1 kHz stereo PCM.
The host test drains the audio ring buffer after each video field. It saves the
samples as WAV files and analyzes the samples it decodes from those files. All
input samples come from DMA fetches, with no AUDxDAT sample injection. The test
calculates the expected output independently of the BLEP tables. The CPU stays
stopped throughout. The test itself initializes the audio path, so guest
application initialization falls outside its scope.

Each run overwrites these disposable outputs.

- `.agent/tmp/audio-spectrum/{pal,ntsc}-filter-{off,on}.wav` contains stereo PCM16.
- `.agent/tmp/audio-spectrum/report.json` records expected and measured
  amplitudes, local spectral peaks, residuals, aliases, and tones appearing on
  the opposite stereo side from their source.

## Stimulus and independent expectations

Each DMA block contains 256 bytes, with a period of 128 CCK and a volume of 64.
The right channel contains harmonics 5, 37, and 83 of the block's repetition
rate. The left contains harmonic 11. In PAL, the frequencies are approximately
541, 4005, and 8984 Hz on the right and 1191 Hz on the left. The different CCK
rate in NTSC changes these frequencies. The test uses actual bus writes to
select the LED filter through CIA-A PA1.

After allowing 441 samples (10 ms) for settling, the test analyzes a 4096-sample
window containing about 93 ms of stationary output. For the Fourier projections,
it applies a periodic Hann window to limit how much tone energy spreads across
the spectrum when the capture cuts across a waveform period. It divides by the
window's coherent gain to correct the amplitude scale. The method follows the
amplitude-scaling and windowing principles in the
[SciPy spectral analysis guide](https://docs.scipy.org/doc/scipy/tutorial/signal.html#spectral-analysis)
and [Hann window documentation](https://docs.scipy.org/doc/scipy/reference/generated/scipy.signal.windows.hann.html).

To calculate an expected amplitude, the test takes a discrete Fourier transform
(DFT) over a full period of the actual quantized input bytes. It then applies the
DAC hold sinc, the 4.9 kHz RC circuit, and the optional 3.275 kHz, Q=0.65 LED
filter from the [filter model](../../../../tools/blep_generator/README.md).
A direct frequency-domain sum evaluates the specified 2048-point, beta-8 Kaiser
sinc with a 21 kHz cutoff. The calculation uses a power series for the Bessel
function, independent of the generator's Chebyshev approximation. The expected
amplitude calculation uses no BLEP table generation, minimum-phase conversion,
or step-history reconstruction.

Holding each DAC value also produces spectral images. In PAL, the high tone on
the right has a lower image near 18.7 kHz, inside the output Nyquist band. The
test checks that image's frequency and filtered amplitude too. It subtracts the
expected components before measuring the power left over. The signal path must
suppress images above Nyquist to prevent them from folding into false audible
tones.

## Acceptance and limits

- Every expected tone and in-band image: amplitude error below 0.5 dB.
- Local peak: within the original bound of about 0.673 Hz; shortening the window
  does not widen the accepted frequency error.
- Residual RMS after fitting expected components: below -35 dB relative to PCM.
- Folded first upper DAC image: below -45 dBc relative to its source tone.
- Foreign-side tone: below -42 dBc, allowing eight-bit quantization harmonics.
- Every generated PCM pair reaches the WAV in order; no ring samples are lost.

The amplitude tolerance accounts for rounded circuit parameters and finite
filter reconstruction. The residual and foreign-tone limits account for source
quantization. The test checks whether the spectral analysis detects deliberately
corrupted PAL samples. It halves the gain, swaps sides, drops one 882-pair block,
and adds an alias at two percent of its source tone. The spectral checks must
reject all four corruptions.

This test validates the implemented signal path against a mathematical filter
model. The evidence covers the exercised DMA workloads. Physical Amiga component
tolerances, exact hardware phase, filter-switching transients, and host playback
remain outside its scope.

## DMA attachment output tests

The command above also runs the [attachment tests](attachment.rs). Use the
following command to run only those two scenarios.

```powershell
python tools/verification/run_rust_tests.py -- cargo test -p machine_loop --test test_audio_spectrum attachment:: -- --nocapture
```

Channel 0 uses DMA to read a looping block of big-endian control words from Chip
RAM. Channel 1 uses DMA to read a quantized sine waveform. After the host starts
playback, changes come through DMA attachment with no further host parameter
writes. The two tests check volume and period independently.

- Volume attachment: 16 -> 48 -> 16, with amplitude ratio 3 within 1% and return
  to the initial amplitude within 1%.
- Period attachment: 256 -> 512 -> 256 CCK, with frequency ratio 2 within 0.2%
  and return to the initial frequency within 0.2%.

Each interval of constant control values contains 16 words at a 4096-CCK source
period. The three plateaus cover both directions of change and the control block's
DMA restart. The test uses the known stimulus clock to place each 1024-sample
analysis window one sixth of a plateau after its expected start, independent of
the observed channel state. This gives DMA time to start, the destination time
to latch the new value, and the filter time to settle.

The tests measure volume by RMS and frequency by interpolating positive-going
zero crossings. The existing Hann Fourier projection checks that the expected
sine accounts for RMS within 2%. Each plateau's frequency must also match its
configured period and video standard within 0.2%. Every window requires at
least twenty zero crossings. Harmonic 37 supplies enough crossings even at the
doubled period. Captures rounded to full frames last approximately 0.10-0.12
seconds for both fixtures.

The tests run in PAL and NTSC with the LED filter off. Channel 0 supplies
modulation data and must stay silent. To check this even during transitions,
the tests require every captured right-side sample to be exactly zero. They
drain the audio buffer by frame to retain every generated PCM pair. They write
and read back
`.agent/tmp/audio-spectrum/{pal,ntsc}-{volume,period}-attachment.wav` before
analyzing the decoded left channel. Measurements are printed with `--nocapture`.
These tests check modulation levels after settling. Channel-state tests check
the timing of attachment parameter transfers.
