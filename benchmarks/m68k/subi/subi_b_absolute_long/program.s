; Measure SUBI.B immediate operand with this destination mode.
operand_data:
    dc.w $3457
entry:
    rept {{instruction_count}}
    subi.b #$57,$200000.l
    move.l d7,d0
    subi.b #$57,$200000.l
    move.l d7,d0
    subi.b #$57,$200000.l
    move.l d7,d0
    subi.b #$57,$200000.l
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
