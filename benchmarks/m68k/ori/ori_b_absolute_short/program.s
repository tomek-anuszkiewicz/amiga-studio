; Measure ORI.B immediate operand with this destination mode.
entry:
    rept {{instruction_count}}
    ori.b #$57,$1000.w
    move.l d7,d0
    ori.b #$57,$1000.w
    move.l d7,d0
    ori.b #$57,$1000.w
    move.l d7,d0
    ori.b #$57,$1000.w
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
