; Measure ANDI.B immediate operand with this destination mode.
entry:
    rept {{instruction_count}}
    andi.b #$57,$1000.w
    move.l d7,d0
    andi.b #$57,$1000.w
    move.l d7,d0
    andi.b #$57,$1000.w
    move.l d7,d0
    andi.b #$57,$1000.w
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
