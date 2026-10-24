; Measure SUB.B D0,D1 within the repeated workload.
entry:
    rept {{instruction_count}}
    sub.b d0,d1
    move.l d6,d7
    sub.b d2,d3
    move.l d6,d7
    sub.b d4,d5
    move.l d6,d7
    sub.b d0,d1
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
