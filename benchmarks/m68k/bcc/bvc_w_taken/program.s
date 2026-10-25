; MOVE.W #$0000,CCR clears N, Z, V and C. BVC.W is taken.
; A missed branch executes the ILLEGAL sentinel.
entry:
    rept {{instruction_count}}
    bvc.w *+6
    dc.w $4AFC
    move.w #$0000,ccr
    bvc.w *+6
    dc.w $4AFC
    move.w #$0000,ccr
    bvc.w *+6
    dc.w $4AFC
    move.w #$0000,ccr
    bvc.w *+6
    dc.w $4AFC
    move.w #$0000,ccr
    endr
    dc.b "M68K_END_MARKER!"
