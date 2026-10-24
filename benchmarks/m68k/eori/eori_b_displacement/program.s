; Measure EORI.B immediate operand with this destination mode.
    dc.w $1357
    dc.w $2469
    dc.w $357B
operand_data:
    dc.w $3457
entry:
    rept {{instruction_count}}
    eori.b #$57,6(a3)
    move.l d7,d0
    eori.b #$57,6(a3)
    move.l d7,d0
    eori.b #$57,6(a3)
    move.l d7,d0
    eori.b #$57,6(a3)
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
