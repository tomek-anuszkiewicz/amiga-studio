; SNE walks byte destinations backward; MOVE.L keeps Z clear.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    sne -(a4)
    move.l d6,d7
    sne -(a4)
    move.l d6,d7
    sne -(a4)
    move.l d6,d7
    sne -(a4)
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
