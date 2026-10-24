; D3 supplies a shift count of 0; D7 restores the long destination.
entry:
    rept {{instruction_count}}
    lsr.l d3,d4
    move.l d7,d4
    lsr.l d3,d4
    move.l d7,d4
    lsr.l d3,d4
    move.l d7,d4
    lsr.l d3,d4
    move.l d7,d4
    endr
    dc.b "M68K_END_MARKER!"
