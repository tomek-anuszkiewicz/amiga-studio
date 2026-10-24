; Measure EORI.B immediate operand with this destination mode.
entry:
    rept {{instruction_count}}
    eori.b #$57,$1000.w
    move.l d7,d0
    eori.b #$57,$1000.w
    move.l d7,d0
    eori.b #$57,$1000.w
    move.l d7,d0
    eori.b #$57,$1000.w
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
