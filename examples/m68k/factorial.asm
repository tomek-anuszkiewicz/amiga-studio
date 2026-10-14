; ==============================================================================
; Factorial Generator with Subroutines & Stack Frames (Motorola 68000)
; ==============================================================================
; Computes factorials 1! through 8! into an array of 32-bit longwords at $002000.
; Demonstrates subroutine calling (BSR.W), stack frame register preservation
; (MOVE.L Dn, -(SP) and MOVE.L (SP)+, Dn), hardware multiplication (MULU.W),
; and subroutine return (RTS).
;
; Target Load Address: $001000
; Execution Mode:      Pure CPU (no Amiga OS or custom chip DMA required)
;
; Stack Configuration:
;   SP (A7) initialized to $008000 (grows downward into RAM)
;
; Expected Output Table at $002000 (8 32-bit longwords):
;   1! = 1        ($00000001)
;   2! = 2        ($00000002)
;   3! = 6        ($00000006)
;   4! = 24       ($00000018)
;   5! = 120      ($00000078)
;   6! = 720      ($000002D0)
;   7! = 5040     ($000013B0)
;   8! = 40320    ($00009D80)
;
; Register Usage:
;   SP (A7) - Hardware stack pointer ($008000)
;   A0      - Output array pointer ($002000)
;   D0      - Parameter N on call; returns N! result
;   D1      - Working multiplier in subroutine (preserved across calls via stack)
;   D2      - Outer loop counter N (1 to 8)
;
; Debugger Tips:
;   - Use F7 (Step Instruction) to step into the BSR subroutine.
;   - Watch the Stack Pointer (SP / A7) decrease by 4 when return address is pushed,
;     and another 4 when D1 is saved.
;   - Open Memory Hex Viewer at $007FF0 to see the return address and saved D1 on the stack!
;   - Watch the factorials populate as 32-bit values at $002000.
; ==============================================================================

            ORG     $001000

start:
            LEA     $008000, SP         ; Initialize Stack Pointer (A7)
            LEA     $002000, A0         ; A0 points to output table
            MOVEQ   #1, D2              ; Start with N = 1

main_loop:
            MOVE.W  D2, D0              ; D0 = parameter N
            BSR.W   fact                ; Call factorial subroutine (pushes PC to SP)
            MOVE.L  D0, (A0)+           ; Store 32-bit N! into table, advance A0
            ADDQ.W  #1, D2              ; N++
            CMPI.W  #9, D2              ; Loop for N = 1..8
            BLT.S   main_loop

halt:
            BRA.S   halt                ; Finished! Spin in idle loop ($60FE)

; ------------------------------------------------------------------------------
; Subroutine: fact
; Computes factorial: D0.L = (D0.W)!
; Preserves D1 on stack.
; ------------------------------------------------------------------------------
fact:
            MOVE.L  D1, -(SP)           ; Preserve D1 on stack ($2F01)
            MOVE.W  D0, D1              ; D1 = N (countdown multiplier)
            MOVEQ   #1, D0              ; D0 = accumulator (1)

fact_loop:
            CMPI.W  #1, D1              ; Base case: if D1 <= 1, done
            BLE.S   fact_done           ; Jump to epilogue
            MULU.W  D1, D0              ; D0.L = D0.W * D1.W (32-bit product)
            SUBQ.W  #1, D1              ; D1--
            BRA.S   fact_loop           ; Next multiplication

fact_done:
            MOVE.L  (SP)+, D1           ; Restore D1 from stack ($221F)
            RTS                         ; Return to caller (pops return address)
