; Measure OR.W D6,$1000.W in the repeated workload.
entry:
    rept {{instruction_count}}
    or.w d6,$1000.w
    move.l d7,d0
    or.w d6,$1000.w
    move.l d7,d0
    or.w d6,$1000.w
    move.l d7,d0
    or.w d6,$1000.w
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
