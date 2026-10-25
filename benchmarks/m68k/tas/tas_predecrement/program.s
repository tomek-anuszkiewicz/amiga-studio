; Four TAS -(A5) targets walk backwards through 100,000 operand bytes.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    tas -(a5)
    move.l d6,d7
    tas -(a5)
    move.l d6,d7
    tas -(a5)
    move.l d6,d7
    tas -(a5)
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
