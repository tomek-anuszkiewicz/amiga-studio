; Measure ADDQ.B #5 with this destination mode.
entry:
    rept {{instruction_count}}
    addq.b #5,$1000.w
    move.l d7,d0
    addq.b #5,$1000.w
    move.l d7,d0
    addq.b #5,$1000.w
    move.l d7,d0
    addq.b #5,$1000.w
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
