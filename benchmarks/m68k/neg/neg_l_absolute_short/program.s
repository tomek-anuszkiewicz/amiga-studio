; Measure NEG.L with the selected destination mode.
entry:
    rept {{instruction_count}}
    neg.l $1000.w
    move.l d7,d0
    neg.l $1000.w
    move.l d7,d0
    neg.l $1000.w
    move.l d7,d0
    neg.l $1000.w
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
