; MOVE.L clears N, Z, V and C. SCS writes $00 to D3.B.
entry:
    rept {{instruction_count}}
    scs d3
    move.l d6,d7
    scs d3
    move.l d6,d7
    scs d3
    move.l d6,d7
    scs d3
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
