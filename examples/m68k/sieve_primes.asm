; ==============================================================================
; Sieve of Eratosthenes (Motorola 68000)
; ==============================================================================
; Computes all prime numbers up to 64 using the classic Sieve algorithm.
;
; Target Load Address: $001000
; Execution Mode:      Pure CPU (no Amiga OS or custom chip DMA required)
;
; Memory Layout:
;   $002000..$00203F: Sieve candidate byte buffer (64 bytes: 0=composite, 1=prime)
;   $002100..$002111: Final sequential list of 18 prime numbers (bytes)
;
; Expected Primes (18 total):
;   2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61
;
; Register Usage:
;   A0 - Sieve candidate buffer pointer ($002000)
;   A1 - Output list pointer ($002100)
;   D0 - Candidate prime p / scanner index
;   D1 - Multiple index being struck out (2*p, 3*p, ...)
;   D7 - Prime counter (18 at completion)
;
; Debugger Tips:
;   - Watch D7 register increment as primes are identified during collection phase.
;   - Open Memory Hex Viewer at $002000 to see composite bytes get cleared to $00.
;   - Open Memory Hex Viewer at $002100 to view the final prime list.
; ==============================================================================

            ORG     $001000

start:
            ; --- Step 1: Initialize 64-byte sieve buffer to 1 ---
            LEA     $002000, A0         ; A0 = base of sieve table
            MOVEQ   #63, D0             ; 64 bytes (63 down to 0)
            MOVEQ   #1, D1              ; Value 1 (candidate prime)

init_loop:
            MOVE.B  D1, 0(A0, D0.W)     ; Sieve[D0] = 1
            DBRA    D0, init_loop

            ; 0 and 1 are not prime
            CLR.B   (A0)                ; Sieve[0] = 0
            CLR.B   1(A0)               ; Sieve[1] = 0

            ; --- Step 2: Strike out multiples of primes ---
            MOVEQ   #2, D0              ; Start with first prime p = 2

sieve_outer:
            TST.B   0(A0, D0.W)         ; Is candidate p still marked as prime?
            BEQ.S   next_p              ; If 0, it is composite, skip

            ; Strike out multiples: D1 = 2*p, 3*p, ... up to 64
            MOVE.W  D0, D1
            ADD.W   D0, D1              ; D1 = 2 * p (first multiple)

strike_loop:
            CMPI.W  #64, D1
            BGE.S   next_p              ; If D1 >= 64, done with this prime
            CLR.B   0(A0, D1.W)         ; Sieve[D1] = 0 (composite)
            ADD.W   D0, D1              ; D1 += p (next multiple)
            BRA.S   strike_loop

next_p:
            ADDQ.W  #1, D0              ; p++
            CMPI.W  #8, D0              ; Check up to sqrt(64) = 8
            BLT.S   sieve_outer

            ; --- Step 3: Collect primes into output list at $002100 ---
            LEA     $002100, A1         ; Destination for prime list
            CLR.W   D7                  ; D7 = prime counter = 0
            MOVEQ   #2, D0              ; Scan candidate range 2..63

collect_loop:
            TST.B   0(A0, D0.W)         ; Is Sieve[D0] == 1?
            BEQ.S   skip_collect        ; If 0, skip
            MOVE.B  D0, (A1)+           ; Store prime byte into (A1)+
            ADDQ.W  #1, D7              ; Increment prime counter

skip_collect:
            ADDQ.W  #1, D0              ; Next number
            CMPI.W  #64, D0
            BLT.S   collect_loop

halt:
            BRA.S   halt                ; Finished! D7 = 18 ($60FE)
