; Measure BCLR with a immediate bit number and absolute_short destination.
entry:
    rept {{instruction_count}}
    bclr #5,$1000.w
    move.l d7,d0
    bclr #5,$1000.w
    move.l d7,d0
    bclr #5,$1000.w
    move.l d7,d0
    bclr #5,$1000.w
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
