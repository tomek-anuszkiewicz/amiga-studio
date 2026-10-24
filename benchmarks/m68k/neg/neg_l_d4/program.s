; Measure NEG.L with the selected destination mode.
entry:
    rept {{instruction_count}}
    neg.l d4
    move.l d7,d0
    neg.l d4
    move.l d7,d0
    neg.l d4
    move.l d7,d0
    neg.l d4
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
