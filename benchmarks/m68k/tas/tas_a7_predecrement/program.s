; Supervisor A7 moves back two bytes for each byte destination.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    tas -(a7)
    move.l d6,d7
    tas -(a7)
    move.l d6,d7
    tas -(a7)
    move.l d6,d7
    tas -(a7)
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
