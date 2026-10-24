; Measure ADDI.B immediate operand with this destination mode.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    addi.b #$57,-(a3)
    move.l d7,d0
    addi.b #$57,-(a3)
    move.l d7,d0
    addi.b #$57,-(a3)
    move.l d7,d0
    addi.b #$57,-(a3)
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
