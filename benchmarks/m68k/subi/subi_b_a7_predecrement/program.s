; Measure SUBI.B immediate operand with this destination mode.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    subi.b #$57,-(a7)
    move.l d7,d0
    subi.b #$57,-(a7)
    move.l d7,d0
    subi.b #$57,-(a7)
    move.l d7,d0
    subi.b #$57,-(a7)
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
