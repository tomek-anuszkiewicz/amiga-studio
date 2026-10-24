; Measure the ABCD byte predecrement path with A7 and A4.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    abcd -(a7),-(a4)
    move.l d7,d0
    abcd -(a7),-(a4)
    move.l d7,d0
    abcd -(a7),-(a4)
    move.l d7,d0
    abcd -(a7),-(a4)
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
