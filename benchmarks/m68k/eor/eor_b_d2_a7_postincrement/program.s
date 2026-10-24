; Exercise EOR.B with SSP postincrement over independent operands.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    eor.b d2,(a7)+
    move.l d7,d1
    eor.b d2,(a7)+
    move.l d7,d1
    eor.b d2,(a7)+
    move.l d7,d1
    eor.b d2,(a7)+
    move.l d7,d1
    endr
    dc.b "M68K_END_MARKER!"
