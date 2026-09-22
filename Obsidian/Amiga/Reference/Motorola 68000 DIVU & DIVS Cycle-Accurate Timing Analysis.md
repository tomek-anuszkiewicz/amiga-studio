---
title: "Motorola 68000 DIVU & DIVS Cycle-Accurate Timing Analysis"
aliases:
  - "DIVU & DIVS Timing Analysis"
  - "68000 Division Cycles"
  - "Pasti DIVU/DIVS Analysis"
tags:
  - amiga
  - reference
  - m68k
  - m68000
  - cpu
  - microcode
  - timing-analysis
  - reverse-engineering
category: "Reference"
subsystem: "m68000"
status: "active"
created: 2026-09-02
updated: 2026-09-14
related:
  - "[CPU Motorola M68000.md](../Design/CPU%20Motorola%20M68000.md)"
  - "[CPU Micro-Step State Machine.md](../Design/CPU%20Micro-Step%20State%20Machine.md)"
properties:
  processor: "Motorola 68000 (NMOS)"
  granularity: "Clock cycles (mcycles * 2)"
  clock_relation: "1 micro-bus cycle (mcycle) = 2 CPU clock cycles"
  instruction_types: ["DIVU.W", "DIVS.W"]
  original_author: "Jorge Cwik (Pasti Project)"
  original_artifact: "Raw C source routines (getDivu68kCycles, getDivs68kCycles)"
---

> [!NOTE] Attribution, Provenance & Original Research
> **Original Reverse-Engineering & Code:** Jorge Cwik (*Pasti* project, Atari ST / M68000 hardware reverse-engineering)  
> **Original Artifact:** Standalone C source implementation functions (`getDivu68kCycles()` and `getDivs68kCycles()`) circulated within the retrocomputing and emulator development community (e.g. Atari-Forum, Hatari, WinUAE, MAME).  
> **Origin of This Document:** The original artifact provided raw C source code routines without comprehensive prose explanations, mathematical derivations, or microcode phase mappings. This document was authored for the Amiga 500 emulator project to expand those C routines into a complete architectural reference, detailing the internal 15-step micro-engine execution, sign-handling penalties, cycle extremes, and integration into cycle-exact emulation.  
> **Community Acknowledgments:** We gratefully acknowledge Jorge Cwik for his groundbreaking reverse-engineering of Motorola 68000 internal execution and timing dynamics.

This document provides an in-depth breakdown and explanation of the cycle-counting routines for the Motorola 68000 `DIVU` (unsigned 32/16 division) and `DIVS` (signed 32/16 division) instructions, reverse-engineered by Jorge Cwik (author of Pasti).

---

## 1. Architectural Background & Terminology

### 1.1 Micro-Cycles (`mcycles`) vs. Master Clock Cycles
* The internal execution unit of the Motorola 68000 operates on micro-machine steps / micro-bus cycles, referred to in the code as `mcycles`.
* Each `mcycle` corresponds to **2 CPU clock cycles** ($f_{\text{master}}$).
* Consequently, the code accumulates micro-cycles into an internal counter `mcycles` and performs `return mcycles * 2` at the end to return standard clock ticks (e.g., at 8 MHz, 1 cycle = 125 ns).

### 1.2 Operand Assumptions
* The routines compute execution time for **register-to-register** division (`DIVU.W Dn, Dm` and `DIVS.W Dn, Dm`).
* If a memory effective address (`<ea>`) mode is used, the effective address calculation and fetch cycles must be added to these baseline totals.
* A return value of `0` denotes an exception case (Division by Zero, Vector #5).

---

## 2. Routine 1: `DIVU` (Unsigned Division)

### 2.1 C Implementation
```c
unsigned getDivu68kCycles( DWORD dividend, WORD divisor)
{
	unsigned mcycles;
	DWORD hdivisor;
	int i;

	if( (WORD) divisor == 0)
		return 0;

	// Overflow
	if( (dividend >> 16) >= divisor)
		return (mcycles = 5) * 2;

	mcycles = 38;
	hdivisor = ((DWORD) divisor) << 16;

	for( i = 0; i < 15; i++)
	{
		DWORD temp;
		temp = dividend;

		dividend <<= 1;

		// If carry from shift
		if( (LONG) temp < 0)
		{
			dividend -= hdivisor;
		}

		else
		{
			mcycles += 2;
			if( dividend >= hdivisor)
			{
				dividend -= hdivisor;
				mcycles--;
			}
		}
	}

	return mcycles * 2;
}
```

### 2.2 Mathematical and Microcode Mechanism
The 68000 micro-engine performs a non-restoring/restoring shift-and-subtract long division step over 15 iterations.
1. **Division by Zero:**
   * Checked immediately: if `divisor == 0`, it aborts returning `0` (the 68000 triggers exception trap processing).
2. **Early Overflow Detection:**
   * A 32-bit dividend divided by a 16-bit divisor produces a valid 16-bit quotient if and only if the high 16 bits of the dividend are strictly less than the divisor (`(dividend >> 16) < divisor`).
   * If `(dividend >> 16) >= divisor`, an overflow condition occurs. The hardware aborts execution after **5 micro-cycles** (10 master clocks).
3. **Base Cost Initialization:**
   * If no overflow occurs, the instruction sets a base cost of `38` micro-cycles (instruction decode, microcode setup, final writeback of quotient/remainder).
   * The divisor is aligned with the upper 16 bits of the 32-bit accumulator (`hdivisor = divisor << 16`).
4. **15-Step Division Loop:**
   * The division loop executes exactly 15 times (for the upper 15 quotient bits; the 16th bit resolution is handled during finalization).
   * In each iteration, `dividend` is shifted left by 1 bit (`dividend <<= 1`).
   * **Case A: Bit 31 was 1 (Carry generated from shift)**
     * `(LONG)temp < 0` tests if the high bit was 1 prior to the shift.
     * When a carry occurs, the partial remainder in the accumulator is guaranteed to be larger than the shifted divisor. Microcode performs an unconditional subtraction `dividend -= hdivisor`.
     * Cost: **+0 extra mcycles** (fast path).
   * **Case B: Bit 31 was 0 (No carry)**
     * The hardware attempts a trial subtraction:
     * First, it takes a penalty of **+2 mcycles** to perform the comparison/trial.
     * If `dividend >= hdivisor`, the trial subtraction succeeds: `dividend -= hdivisor`, and **1 mcycle is refunded** (`mcycles--`), resulting in a net penalty of **+1 mcycle**.
     * If `dividend < hdivisor`, subtraction does not occur, leaving the net penalty at **+2 mcycles**.
5. **Cycle Extremes:**
   * **Overflow:** $5 \times 2 = 10$ cycles.
   * **Best Case:** When carry occurs or trial subtraction succeeds on every cycle:
     $$\text{Base } 38 + 15 \times 0 = 38 \text{ mcycles} \implies 76 \text{ cycles}.$$
   * **Worst Case:** When trial subtraction fails on every step ($+2$ per iteration):
     $$\text{Base } 38 + 15 \times 2 = 68 \text{ mcycles} \implies 136 \text{ cycles}.$$

---

## 3. Routine 2: `DIVS` (Signed Division)

### 3.1 C Implementation
```c
unsigned getDivs68kCycles( LONG dividend, SHORT divisor)
{
	unsigned mcycles;
	unsigned aquot;
	int i;

	if( (SHORT) divisor == 0)
		return 0;

	mcycles = 6;

	if( dividend < 0)
		mcycles++;

	// Check for absolute overflow
	if( ((DWORD) abs( dividend) >> 16) >= (WORD) abs( divisor))
	{
		return (mcycles + 2) * 2;
	}

	// Absolute quotient
	aquot = (DWORD) abs( dividend) / (WORD) abs( divisor);

	mcycles += 55;

	if( divisor >= 0)
	{
		if( dividend >= 0)
			mcycles--;
		else
			mcycles++;
	}

	// Count 15 msbits in absolute of quotient

	for( i = 0; i < 15; i++)
	{
		if( (SHORT) aquot >= 0)
			mcycles++;
		aquot <<= 1;
	}

	return mcycles * 2;
}
```

### 3.2 Mathematical and Microcode Mechanism
The 68000 executes `DIVS` by converting both dividend and divisor into their absolute values, executing the unsigned core division, adjusting signs, and checking for 16-bit signed range overflow ($-32768$ to $+32767$).

1. **Division by Zero:**
   * Returns `0` if `divisor == 0`.
2. **Initial Sign Evaluation & Absolute Overflow:**
   * Base microcode setup begins at `mcycles = 6`.
   * **Dividend Negation Penalty:** If `dividend < 0`, hardware must complement it to positive, adding **+1 mcycle** (`mcycles++`).
   * **Absolute Overflow Check:** If `|dividend| >> 16 >= |divisor|`, the operation exceeds 16-bit storage unconditionally.
     * The hardware aborts after an additional 2 micro-cycles.
     * Resulting cycle count:
       * Positive dividend overflow: $(6 + 2) \times 2 = 16$ cycles.
       * Negative dividend overflow: $(6 + 1 + 2) \times 2 = 18$ cycles.
3. **Core Processing & Sign Correction Penalties:**
   * If absolute overflow does not occur, execution continues by adding a base of **+55 mcycles**.
   * Sign combination adjustment:
     * If `divisor >= 0` and `dividend >= 0` (both positive): **$-1$ mcycle** (no post-negation needed).
     * If `divisor >= 0` and `dividend < 0` (mixed sign): **$+1$ mcycle**.
     * If `divisor < 0`: the branch leaves `mcycles` unchanged ($+0$).
4. **Quotient Bit Inspection (Micro-loop):**
   * Rather than simulating the non-restoring ALU state directly as in `DIVU`, Jorge Cwik's routine computes the mathematical absolute quotient:
     ```c
     aquot = (DWORD) abs(dividend) / (WORD) abs(divisor);
     ```
   * The 68000 micro-engine step penalty correlates directly to the individual bits of the absolute quotient.
   * A 15-iteration loop shifts the absolute quotient left bit-by-bit:
     ```c
     if ((SHORT)aquot >= 0)
         mcycles++;
     aquot <<= 1;
     ```
   * Testing `(SHORT)aquot >= 0` checks whether bit 15 (sign bit of the 16-bit word) is `0`.
   * For every `0` bit encountered among the 15 most significant quotient bits, an extra **+1 mcycle** penalty is incurred.
5. **Cycle Extremes:**
   * **Absolute Overflow:** $16$ or $18$ cycles.
   * **Worst Case:** 15 zeroes in the quotient bits with worst-case sign penalties:
     $$\text{Total} = 156 \text{ cycles}.$$
   * **Best Case (without signed overflow):** $122$ cycles.
   * **Best Case (with signed overflow):** $120$ cycles.

---

## 4. Summary Comparison Table

| Feature | `DIVU` (Unsigned) | `DIVS` (Signed) |
| :--- | :--- | :--- |
| **Overflow Detection** | Immediate 16-bit check | Checks absolute values first |
| **Overflow Timing** | Strictly 10 cycles | 16 cycles (pos. dividend), 18 cycles (neg. dividend) |
| **Loop Iterations** | 15 iterations | 15 iterations |
| **Loop Metric** | Accumulator carry & remainder vs divisor | Quotient bit pattern (`0` bits add cycle) |
| **Minimum Cycles (Valid)**| 76 cycles | 122 cycles (120 with signed overflow) |
| **Maximum Cycles** | 136 cycles | 156 cycles |

---

## 5. References & Upstream Ground Truth

- **Pasti Project (Jorge Cwik):** [http://pasti.fxatari.com](http://pasti.fxatari.com) — Original home of the Pasti imaging engine and 68000 cycle-accuracy research.
- **Upstream Emulator Implementations:** Jorge Cwik's division algorithms serve as the golden standard adopted by [Hatari](https://hatari.tuxfamily.org/), [WinUAE](https://www.winuae.net/), and [MAME](https://www.mamedev.org/).
- **Architecture Design Specifications:**
  - [CPU Motorola M68000.md](../Design/CPU%20Motorola%20M68000.md): Architectural design document for the living Rust CPU core in this repository.
  - [CPU Micro-Step State Machine.md](../Design/CPU%20Micro-Step%20State%20Machine.md): Micro-step archetype, bus cycles, and execution phases.
- **Living Rust CPU Implementations:**
  - [`crates/cpu/src/instructions/divu.rs`](../../../crates/cpu/src/instructions/divu.rs): Rust implementation of `DIVU` and `calc_divu_internal_clocks()`.
  - [`crates/cpu/src/instructions/divs.rs`](../../../crates/cpu/src/instructions/divs.rs): Rust implementation of `DIVS` and `calc_divs_internal_clocks()`.

