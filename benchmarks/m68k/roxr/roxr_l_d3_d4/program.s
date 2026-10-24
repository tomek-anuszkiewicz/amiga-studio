; D3 supplies a three-bit count; D7 restores the long destination.
entry:
    rept {{instruction_count}}
    roxr.l d3,d4
    move.l d7,d4
    roxr.l d3,d4
    move.l d7,d4
    roxr.l d3,d4
    move.l d7,d4
    roxr.l d3,d4
    move.l d7,d4
    endr
    dc.b "M68K_END_MARKER!"
