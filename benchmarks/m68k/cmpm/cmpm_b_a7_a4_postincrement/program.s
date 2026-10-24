; SSP source advances by two bytes; destination advances by one.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    cmpm.b (a7)+,(a4)+
    move.l d6,d2
    cmpm.b (a7)+,(a4)+
    move.l d6,d2
    cmpm.b (a7)+,(a4)+
    move.l d6,d2
    cmpm.b (a7)+,(a4)+
    move.l d6,d2
    endr
    dc.b "M68K_END_MARKER!"
