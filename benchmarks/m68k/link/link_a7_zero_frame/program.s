; LINK A7 creates nested zero-size frames, with MOVE.L between targets.
entry:
    rept {{instruction_count}}
    link a7,#0
    move.l d6,d7
    link a7,#0
    move.l d6,d7
    link a7,#0
    move.l d6,d7
    link a7,#0
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
