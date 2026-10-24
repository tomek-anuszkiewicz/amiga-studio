; Measure EOR.W D6,$1000.W in the repeated workload.
entry:
    rept {{instruction_count}}
    eor.w d6,$1000.w
    move.l d7,d0
    eor.w d6,$1000.w
    move.l d7,d0
    eor.w d6,$1000.w
    move.l d7,d0
    eor.w d6,$1000.w
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
