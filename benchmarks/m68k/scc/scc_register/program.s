; MOVE.L clears N, Z, V and C. SCC writes $FF to D3.B.
entry:
    rept {{instruction_count}}
    scc d3
    move.l d6,d7
    scc d3
    move.l d6,d7
    scc d3
    move.l d6,d7
    scc d3
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
