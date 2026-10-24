; Measure NEG.B with the selected destination mode.
entry:
    rept {{instruction_count}}
    neg.b $1000.w
    move.l d7,d0
    neg.b $1000.w
    move.l d7,d0
    neg.b $1000.w
    move.l d7,d0
    neg.b $1000.w
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
