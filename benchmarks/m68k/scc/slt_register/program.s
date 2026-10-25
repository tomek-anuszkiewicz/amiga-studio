; MOVE.L clears N, Z, V and C. SLT writes $00 to D3.B.
entry:
    rept {{instruction_count}}
    slt d3
    move.l d6,d7
    slt d3
    move.l d6,d7
    slt d3
    move.l d6,d7
    slt d3
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
