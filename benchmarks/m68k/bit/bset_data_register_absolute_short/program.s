; Measure BSET with a data_register bit number and absolute_short destination.
entry:
    rept {{instruction_count}}
    bset d3,$1000.w
    move.l d7,d0
    bset d3,$1000.w
    move.l d7,d0
    bset d3,$1000.w
    move.l d7,d0
    bset d3,$1000.w
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
