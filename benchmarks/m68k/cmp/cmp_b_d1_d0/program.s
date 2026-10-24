; Measure CMP.B D0,D1 without changing the destination.
entry:
    rept {{instruction_count}}
    cmp.b d0,d1
    move.l d6,d7
    cmp.b d2,d3
    move.l d6,d7
    cmp.b d4,d5
    move.l d6,d7
    cmp.b d0,d1
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
