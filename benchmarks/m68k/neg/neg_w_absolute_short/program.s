; Measure NEG.W with the selected destination mode.
entry:
    rept {{instruction_count}}
    neg.w $1000.w
    move.l d7,d0
    neg.w $1000.w
    move.l d7,d0
    neg.w $1000.w
    move.l d7,d0
    neg.w $1000.w
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
