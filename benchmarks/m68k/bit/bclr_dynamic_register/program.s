; Use three dynamic bit numbers and repeat the first register pair.
entry:
    rept {{instruction_count}}
    bclr d0,d1
    move.l d6,d7
    bclr d2,d3
    move.l d6,d7
    bclr d4,d5
    move.l d6,d7
    bclr d0,d1
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
