; ==============================================================================
; Fibonacci Sequence Generator (Motorola 68000)
; ==============================================================================
; Computes the first 16 Fibonacci numbers into working RAM starting at $002000.
;
; Target Load Address: $001000
; Execution Mode:      Pure CPU (no Amiga OS or custom chip DMA required)
;
; Register Usage:
;   A0 - Output table pointer (starts at $002000)
;   D0 - F(n-2) Fibonacci term
;   D1 - F(n-1) Fibonacci term
;   D2 - F(n) current term = D0 + D1
;   D3 - Loop counter (14 iterations remaining, counts down to -1 via DBRA)
;
; Output Memory ($002000..$00201F, 16 16-bit words):
;   $0000, $0001, $0001, $0002, $0003, $0005, $0008, $000D,
;   $0015, $0022, $0037, $0059, $0090, $00E9, $0179, $0262
;
; Debugger Tips:
;   - Set PC to $001000 and step through with F7 (Step Instruction) or F8 (Step CCK).
;   - Set a breakpoint at $001014 (fib_loop) to watch each iteration update D2.
;   - Open Memory Hex Viewer at address $002000 to watch numbers populate.
; ==============================================================================

            ORG     $001000

start:
            LEA     $002000, A0         ; A0 points to output buffer
            CLR.W   D0                  ; D0 = F(0) = 0
            MOVE.W  #1, D1              ; D1 = F(1) = 1
            MOVE.W  D0, (A0)+           ; Store F(0) at $002000, A0 -> $002002
            MOVE.W  D1, (A0)+           ; Store F(1) at $002002, A0 -> $002004
            MOVE.W  #13, D3             ; 14 iterations for remaining numbers

fib_loop:
            MOVE.W  D0, D2              ; D2 = D0 (F(n-2))
            ADD.W   D1, D2              ; D2 = D0 + D1 (F(n))
            MOVE.W  D2, (A0)+           ; Store F(n) into memory table, advance A0
            MOVE.W  D1, D0              ; Advance: F(n-2) = old F(n-1)
            MOVE.W  D2, D1              ; Advance: F(n-1) = current F(n)
            DBRA    D3, fib_loop        ; Decrement D3; if D3 != -1 branch to fib_loop

halt:
            BRA.S   halt                ; Finished! Spin in place ($60FE)
