; MOVE.L clears N, Z, V and C. SLE writes $00 to D3.B.
entry:
    rept {{instruction_count}}
    sle d3
    move.l d6,d7
    sle d3
    move.l d6,d7
    sle d3
    move.l d6,d7
    sle d3
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
