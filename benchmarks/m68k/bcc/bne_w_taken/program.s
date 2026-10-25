; Z remains clear after each helper. A missed branch executes the ILLEGAL sentinel.
entry:
    rept {{instruction_count}}
    bne.w *+6
    dc.w $4AFC
    move.w #$0000,ccr
    bne.w *+6
    dc.w $4AFC
    move.w #$0000,ccr
    bne.w *+6
    dc.w $4AFC
    move.w #$0000,ccr
    bne.w *+6
    dc.w $4AFC
    move.w #$0000,ccr
    endr
    dc.b "M68K_END_MARKER!"
