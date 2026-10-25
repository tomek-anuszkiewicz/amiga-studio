; MOVE.L clears N, Z, V and C. SVS writes $00 to D3.B.
entry:
    rept {{instruction_count}}
    svs d3
    move.l d6,d7
    svs d3
    move.l d6,d7
    svs d3
    move.l d6,d7
    svs d3
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
