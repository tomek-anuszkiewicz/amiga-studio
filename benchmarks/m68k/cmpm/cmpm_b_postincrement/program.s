; Use byte width for a second CMPM path; addressing stays postincrement.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    cmpm.b (a2)+,(a4)+
    move.l d7,d0
    cmpm.b (a2)+,(a4)+
    move.l d7,d0
    cmpm.b (a2)+,(a4)+
    move.l d7,d0
    cmpm.b (a2)+,(a4)+
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
