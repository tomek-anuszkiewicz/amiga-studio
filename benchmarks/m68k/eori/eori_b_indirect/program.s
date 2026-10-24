; Measure EORI.B immediate operand with this destination mode.
operand_data:
    dc.w $3457
entry:
    rept {{instruction_count}}
    eori.b #$57,(a3)
    move.l d7,d0
    eori.b #$57,(a3)
    move.l d7,d0
    eori.b #$57,(a3)
    move.l d7,d0
    eori.b #$57,(a3)
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
