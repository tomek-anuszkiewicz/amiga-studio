; D3 supplies a shift count of 8; D7 restores the byte destination.
entry:
    rept {{instruction_count}}
    roxr.b d3,d4
    move.l d7,d4
    roxr.b d3,d4
    move.l d7,d4
    roxr.b d3,d4
    move.l d7,d4
    roxr.b d3,d4
    move.l d7,d4
    endr
    dc.b "M68K_END_MARKER!"
