; Measure CMP.L D0,D1 without changing the destination.
entry:
    rept {{instruction_count}}
    cmp.l d0,d1
    move.l d6,d7
    cmp.l d2,d3
    move.l d6,d7
    cmp.l d4,d5
    move.l d6,d7
    cmp.l d0,d1
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
