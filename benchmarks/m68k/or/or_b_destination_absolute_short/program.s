; Measure OR.B D6,$1000.W in the repeated workload.
entry:
    rept {{instruction_count}}
    or.b d6,$1000.w
    move.l d7,d0
    or.b d6,$1000.w
    move.l d7,d0
    or.b d6,$1000.w
    move.l d7,d0
    or.b d6,$1000.w
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
