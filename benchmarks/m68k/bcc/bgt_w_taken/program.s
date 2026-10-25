; MOVE.W #$0000,CCR clears N, Z, V and C. BGT.W is taken.
; A missed branch executes the ILLEGAL sentinel.
entry:
    rept {{instruction_count}}
    bgt.w *+6
    dc.w $4AFC
    move.w #$0000,ccr
    bgt.w *+6
    dc.w $4AFC
    move.w #$0000,ccr
    bgt.w *+6
    dc.w $4AFC
    move.w #$0000,ccr
    bgt.w *+6
    dc.w $4AFC
    move.w #$0000,ccr
    endr
    dc.b "M68K_END_MARKER!"
