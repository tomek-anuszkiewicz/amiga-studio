; MOVE.W #$0000,CCR clears N, Z, V and C. BLT.W falls through.
; An incorrect taken path reaches an ILLEGAL sentinel at the first
; target or after the end marker at the final target.
    dc.w $4AFC
entry:
    rept {{instruction_count}}
    blt.w *-2
    move.w #$0000,ccr
    blt.w *+8
    move.w #$0000,ccr
    blt.w *+8
    move.w #$0000,ccr
    blt.w *+24
    move.w #$0000,ccr
    endr
    dc.b "M68K_END_MARKER!"
    dc.w $4AFC
