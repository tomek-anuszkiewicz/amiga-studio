; Measure CMPI.B immediate operand with this destination mode.
operand_data:
    dc.w $3457
entry:
    rept {{instruction_count}}
    cmpi.b #$57,$200000.l
    move.l d7,d0
    cmpi.b #$57,$200000.l
    move.l d7,d0
    cmpi.b #$57,$200000.l
    move.l d7,d0
    cmpi.b #$57,$200000.l
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
