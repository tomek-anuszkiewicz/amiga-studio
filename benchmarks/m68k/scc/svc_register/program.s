; MOVE.L clears N, Z, V and C. SVC writes $FF to D3.B.
entry:
    rept {{instruction_count}}
    svc d3
    move.l d6,d7
    svc d3
    move.l d6,d7
    svc d3
    move.l d6,d7
    svc d3
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
