# M68000 Standalone Debugger Test Programs

The headless debugger tests load these Motorola 68000 programs at `$001000`
and check the results. The programs run entirely on the CPU and need no
Kickstart ROM or custom chip DMA. They end in a loop at `halt` (`BRA.S halt`,
machine code `$60FE`).

## 1. Run the sample programs

Run from the repository root. The test creates each binary and checks the
guest results.

```powershell
python tools/verification/verify.py test rust crate debugger suite test_sample_binaries
```

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
- **Description.** Copies an unsorted array of eight 16-bit words to `$002000`, then uses bubble sort to reorder the words at that address.
- **Entry Address:** `$001000`
- **Output Area:** `$002000` (16 bytes / 8 words)
- **Initial Array:** `$0042, $0010, $0099, $0003, $0077, $0025, $0001, $0050`
- **Sorted Array:**
  ```
  $002000: 0001 0003 0010 0025 0042 0050 0077 0099
  ```
- **Registers to Watch:** `A0` (pointer advancing through the array), `D0` / `D1` (compared pair), `D6` (inner pass), `D7` (outer pass).
- **Suggested Breakpoints:** `$001028` hits only when the program swaps the values in place.
- **Debugger Features Highlighted:** The Hex Viewer highlights changed RAM in red. Use CPU history (`Shift+F10`) to see earlier register states. This rewinds CPU state only and leaves memory at its current value.

---

### 3. `sieve_primes.asm` / `sieve_primes.bin`
- **Description.** Uses the Sieve of Eratosthenes to find all prime numbers under 64.
- **Entry Address:** `$001000`
- **Output Area:**
  - `$002000..$00203F`: 64-byte sieve buffer ($00 = composite, $01 = prime).
  - `$002100..$002111`: Sequential list of prime bytes (18 primes total).
- **Expected Primes:** `2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61`
- **Registers to Watch:** `D0` (candidate $p$), `D1` (current multiple), `D7` (prime count = 18 at finish).
- **Suggested Breakpoints:** `$00102A` (composite strikeout), `$00104A` (prime collection), `$001056` (halt).

---

### 4. `string_reverse.asm` / `string_reverse.bin`
- **Description:** Shows how to manipulate strings, transfer bytes and detect a palindrome.
  1. Reverses `"AMIGA 500 RULEZ!"` into `$002040` (`"!ZELUR 005 AGIMA"`).
  2. Checks whether `"RACECAR"` is a palindrome. It sets `D0 = 1` for true or `0` for false.
- **Entry Address:** `$001000`
- **Output Area:**
  - `$002000`: Original string `"AMIGA 500 RULEZ!"`
  - `$002040`: Reversed string `"!ZELUR 005 AGIMA"`
- **Registers to Watch:** `D0` (palindrome result: 1 at halt), `A1` / `A2` (reverse pointers), `A3` / `A4` (palindrome pointers).
- **Debugger Features Highlighted:** Select **"ASCII"** in the Memory Hex Viewer to read the strings in ASCII.

---

### 5. `factorial.asm` / `factorial.bin`
- **Description.** Demonstrates a subroutine call and the call stack. It computes factorials $1!$ through $8!$ as 32-bit integers at `$002000`.
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
- **Debugger Features Highlighted:** Step into `BSR.W` with **F10**, inspect the return address and saved registers pushed to the stack at `$007FF0..$007FFF`, and watch `MULU.W` calculate 32-bit products.
