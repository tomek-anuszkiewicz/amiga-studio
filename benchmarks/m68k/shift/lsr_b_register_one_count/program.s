; D3 supplies a shift count of 1; D7 restores the byte destination.
entry:
    rept {{instruction_count}}
    lsr.b d3,d4
    move.l d7,d4
    lsr.b d3,d4
    move.l d7,d4
    lsr.b d3,d4
    move.l d7,d4
    lsr.b d3,d4
    move.l d7,d4
    endr
    dc.b "M68K_END_MARKER!"
