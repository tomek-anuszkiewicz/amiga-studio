; MOVE.L clears N, Z, V and C. ST writes $FF to D3.B.
entry:
    rept {{instruction_count}}
    st d3
    move.l d6,d7
    st d3
    move.l d6,d7
    st d3
    move.l d6,d7
    st d3
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
