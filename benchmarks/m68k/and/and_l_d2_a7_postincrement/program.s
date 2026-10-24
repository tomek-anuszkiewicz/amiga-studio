; Exercise AND.L with SSP postincrement over independent operands.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    and.l d2,(a7)+
    move.l d7,d1
    and.l d2,(a7)+
    move.l d7,d1
    and.l d2,(a7)+
    move.l d7,d1
    and.l d2,(a7)+
    move.l d7,d1
    endr
    dc.b "M68K_END_MARKER!"
