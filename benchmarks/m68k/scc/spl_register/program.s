; MOVE.L clears N, Z, V and C. SPL writes $FF to D3.B.
entry:
    rept {{instruction_count}}
    spl d3
    move.l d6,d7
    spl d3
    move.l d6,d7
    spl d3
    move.l d6,d7
    spl d3
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
