; Both operands use SSP; each byte access advances A7 by two bytes.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    cmpm.b (a7)+,(a7)+
    move.l d5,d2
    cmpm.b (a7)+,(a7)+
    move.l d5,d2
    cmpm.b (a7)+,(a7)+
    move.l d5,d2
    cmpm.b (a7)+,(a7)+
    move.l d5,d2
    endr
    dc.b "M68K_END_MARKER!"
