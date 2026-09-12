# M68000 Standalone Debugger Test Programs

This directory contains standalone Motorola 68000 test programs designed specifically for interactive debugging, stepping, and inspection in the **Amiga 500 Developer Studio** (native GUI and WebAssembly).

All programs are **100% pure CPU execution**: they require no Amiga OS, Kickstart ROM, or custom chip DMA. They load into RAM at `$001000` and end with an idle spin loop (`halt: BRA.S halt`, machine code `$60FE`), ensuring continuous running (`F5`) halts safely without faults.

---

## 1. Quick Start: Loading into Debugger

### In the Desktop / Web GUI:
1. Click **"Load Binary"** (or press the Binary injection button in the toolbar).
2. Select any `.bin` file from this folder (`tests/bin/*.bin`).
3. Set the injection target address to `$001000` (default) and ensure **"Reset PC to injection address"** is checked.
4. Click **"Inject & Reset PC"**.
5. Step through instructions with **F7** (Step Instruction), Color Clock phases with **F8** (Step CCK), or run with **F5**.

### Command-Line Launch:
```powershell
cargo run -p gui -- --load tests/bin/fibonacci.bin
```

---

## 2. Program Catalog

### 1. `fibonacci.asm` / `fibonacci.bin`
- **Description:** Generates the first 16 Fibonacci numbers ($F_0$ through $F_{15}$) in 16-bit word format.
- **Entry Address:** `$001000`
- **Output Area:** `$002000` (32 bytes / 16 words)
- **Expected Output:**
  ```
  $002000: 0000 0001 0001 0002 0003 0005 0008 000D
  $002010: 0015 0022 0037 0059 0090 00E9 0179 0262
  ```
- **Registers to Watch:** `D0` ($F_{n-2}$), `D1` ($F_{n-1}$), `D2` ($F_n$), `D3` (loop counter), `A0` (table pointer).
- **Suggested Breakpoints:** `$001014` (loop top), `$001022` (halt).

---

### 2. `bubble_sort.asm` / `bubble_sort.bin`
- **Description:** Copies an unsorted array of eight 16-bit words to `$002000` and sorts them in-place using bubble sort.
- **Entry Address:** `$001000`
- **Output Area:** `$002000` (16 bytes / 8 words)
- **Initial Array:** `42, 10, 99, 3, 77, 25, 1, 50`
- **Sorted Array:**
  ```
  $002000: 0001 0003 0010 0025 0042 0050 0077 0099
  ```
- **Registers to Watch:** `A0` (walking pointer), `D0` / `D1` (compared pair), `D6` (inner pass), `D7` (outer pass).
- **Suggested Breakpoints:** `$001028` (hits only when an in-place swap occurs!).
- **Debugger Features Highlighted:** In-place RAM modification diffs (red highlights in Hex Viewer) and Temporal History rewind (`Alt+Left`) to replay previous sort passes.

---

### 3. `sieve_primes.asm` / `sieve_primes.bin`
- **Description:** Sieve of Eratosthenes finding all prime numbers under 64.
- **Entry Address:** `$001000`
- **Output Area:**
  - `$002000..$00203F`: 64-byte sieve buffer ($00 = composite, $01 = prime).
  - `$002100..$002111`: Sequential list of prime bytes (18 primes total).
- **Expected Primes:** `2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61`
- **Registers to Watch:** `D0` (candidate $p$), `D1` (current multiple), `D7` (prime count = 18 at finish).
- **Suggested Breakpoints:** `$00102A` (composite strikeout), `$00104A` (prime collection), `$001056` (halt).

---

### 4. `string_reverse.asm` / `string_reverse.bin`
- **Description:** Demonstrates string manipulations, byte transfers, and palindrome detection:
  1. Reverses `"AMIGA 500 RULEZ!"` into `$002040` (`"!ZELUR 005 AGIMA"`).
  2. Tests `"RACECAR"` for palindrome; sets `D0 = 1` (true) or `0` (false).
- **Entry Address:** `$001000`
- **Output Area:**
  - `$002000`: Original string `"AMIGA 500 RULEZ!"`
  - `$002040`: Reversed string `"!ZELUR 005 AGIMA"`
- **Registers to Watch:** `D0` (palindrome result: 1 at halt), `A1` / `A2` (reverse pointers), `A3` / `A4` (palindrome pointers).
- **Debugger Features Highlighted:** Enable the **"ASCII"** checkbox in the Memory Hex Viewer to read the strings directly in ASCII!

---

### 5. `factorial.asm` / `factorial.bin`
- **Description:** Subroutine and call stack demonstration. Computes factorials $1!$ through $8!$ as 32-bit integers at `$002000`.
- **Entry Address:** `$001000`
- **Stack Pointer:** `SP` (`A7`) initialized to `$008000`.
- **Output Area:** `$002000` (32 bytes / 8 32-bit longs)
- **Expected Factorials:**
  - $1! = 1$ (`$00000001`)
  - $2! = 2$ (`$00000002`)
  - $3! = 6$ (`$00000006`)
  - $4! = 24$ (`$00000018`)
  - $5! = 120$ (`$00000078`)
  - $6! = 720$ (`$000002D0`)
  - $7! = 5040$ (`$000013B0`)
  - $8! = 40320$ (`$00009D80`)
- **Registers to Watch:** `SP` (`A7`, decrements on `BSR` and `MOVE.L D1, -(SP)`), `D0` (return value), `D2` (parameter $N$).
- **Debugger Features Highlighted:** Step into `BSR.W` with **F7**, inspect the return address and saved registers pushed to the stack at `$007FF0..$007FFF`, and watch `MULU.W` calculate 32-bit products.
