; The target walks backward through independent byte destinations.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    scc -(a4)
    move.l d6,d7
    scc -(a4)
    move.l d6,d7
    scc -(a4)
    move.l d6,d7
    scc -(a4)
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
