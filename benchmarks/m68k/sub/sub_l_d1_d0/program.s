; Measure SUB.L D0,D1 within the repeated workload.
entry:
    rept {{instruction_count}}
    sub.l d0,d1
    move.l d6,d7
    sub.l d2,d3
    move.l d6,d7
    sub.l d4,d5
    move.l d6,d7
    sub.l d0,d1
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
