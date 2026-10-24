; Measure SUB.B D6,$1000.W within the repeated workload.
entry:
    rept {{instruction_count}}
    sub.b d6,$1000.w
    move.l d7,d0
    sub.b d6,$1000.w
    move.l d7,d0
    sub.b d6,$1000.w
    move.l d7,d0
    sub.b d6,$1000.w
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
