; D3 supplies a shift count of 63; D7 restores the byte destination.
entry:
    rept {{instruction_count}}
    roxl.b d3,d4
    move.l d7,d4
    roxl.b d3,d4
    move.l d7,d4
    roxl.b d3,d4
    move.l d7,d4
    roxl.b d3,d4
    move.l d7,d4
    endr
    dc.b "M68K_END_MARKER!"
