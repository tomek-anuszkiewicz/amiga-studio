; Walk initialized word data forward with A7 byte postincrement.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    and.b d3,(a7)+
    move.l d7,d0
    and.b d3,(a7)+
    move.l d7,d0
    and.b d3,(a7)+
    move.l d7,d0
    and.b d3,(a7)+
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
