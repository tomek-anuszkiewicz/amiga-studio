; MOVE.L clears N, Z, V and C. SF writes $00 to D3.B.
entry:
    rept {{instruction_count}}
    sf d3
    move.l d6,d7
    sf d3
    move.l d6,d7
    sf d3
    move.l d6,d7
    sf d3
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
