; Measure SUBQ.L #5 with this destination mode.
entry:
    rept {{instruction_count}}
    subq.l #5,$1000.w
    move.l d7,d0
    subq.l #5,$1000.w
    move.l d7,d0
    subq.l #5,$1000.w
    move.l d7,d0
    subq.l #5,$1000.w
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
