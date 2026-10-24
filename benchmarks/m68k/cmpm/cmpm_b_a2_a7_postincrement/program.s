; Source advances by one byte; SSP destination advances by two.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    cmpm.b (a2)+,(a7)+
    move.l d3,d6
    cmpm.b (a2)+,(a7)+
    move.l d3,d6
    cmpm.b (a2)+,(a7)+
    move.l d3,d6
    cmpm.b (a2)+,(a7)+
    move.l d3,d6
    endr
    dc.b "M68K_END_MARKER!"
