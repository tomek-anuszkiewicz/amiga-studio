; Four LINK A2,#0 targets create nested frames, with MOVE.L between targets.
entry:
    rept {{instruction_count}}
    link a2,#0
    move.l d6,d7
    link a2,#0
    move.l d6,d7
    link a2,#0
    move.l d6,d7
    link a2,#0
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
