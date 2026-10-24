; D3 supplies a shift count of 8; D7 restores the word destination.
entry:
    rept {{instruction_count}}
    lsr.w d3,d4
    move.l d7,d4
    lsr.w d3,d4
    move.l d7,d4
    lsr.w d3,d4
    move.l d7,d4
    lsr.w d3,d4
    move.l d7,d4
    endr
    dc.b "M68K_END_MARKER!"
