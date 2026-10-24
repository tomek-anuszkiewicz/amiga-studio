; Measure SUB.L D6,$1000.W within the repeated workload.
entry:
    rept {{instruction_count}}
    sub.l d6,$1000.w
    move.l d7,d0
    sub.l d6,$1000.w
    move.l d7,d0
    sub.l d6,$1000.w
    move.l d7,d0
    sub.l d6,$1000.w
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
