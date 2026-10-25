; The target walks backward through independent byte destinations.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    sle -(a4)
    move.l d6,d7
    sle -(a4)
    move.l d6,d7
    sle -(a4)
    move.l d6,d7
    sle -(a4)
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
