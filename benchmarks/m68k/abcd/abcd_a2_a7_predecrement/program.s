; Measure the ABCD byte predecrement path with A2 and A7.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    abcd -(a2),-(a7)
    move.l d7,d0
    abcd -(a2),-(a7)
    move.l d7,d0
    abcd -(a2),-(a7)
    move.l d7,d0
    abcd -(a2),-(a7)
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
