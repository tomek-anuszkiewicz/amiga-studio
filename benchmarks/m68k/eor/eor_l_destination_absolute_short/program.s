; Measure EOR.L D6,$1000.W in the repeated workload.
entry:
    rept {{instruction_count}}
    eor.l d6,$1000.w
    move.l d7,d0
    eor.l d6,$1000.w
    move.l d7,d0
    eor.l d6,$1000.w
    move.l d7,d0
    eor.l d6,$1000.w
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
