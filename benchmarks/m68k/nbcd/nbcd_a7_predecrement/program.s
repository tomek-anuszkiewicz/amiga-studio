; Measure NBCD memory addressing with packed decimal operands.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    nbcd -(a7)
    move.l d7,d0
    nbcd -(a7)
    move.l d7,d0
    nbcd -(a7)
    move.l d7,d0
    nbcd -(a7)
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
