; Four TAS (A6)+ targets, each followed by a MOVE helper.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    tas (a6)+
    move.l d6,d7
    tas (a6)+
    move.l d6,d7
    tas (a6)+
    move.l d6,d7
    tas (a6)+
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
