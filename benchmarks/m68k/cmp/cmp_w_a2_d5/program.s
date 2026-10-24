; Measure CMP.W A0,D4 without changing the destination.
entry:
    rept {{instruction_count}}
    cmp.w a0,d4
    move.l d1,d5
    cmp.w a1,d5
    move.l d2,d6
    cmp.w a2,d6
    move.l d3,d7
    cmp.w a3,d7
    move.l d0,d4
    endr
    dc.b "M68K_END_MARKER!"
