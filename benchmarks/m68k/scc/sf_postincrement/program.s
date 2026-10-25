; The target walks forward through independent byte destinations.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    sf (a2)+
    move.l d6,d7
    sf (a2)+
    move.l d6,d7
    sf (a2)+
    move.l d6,d7
    sf (a2)+
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
