; Measure the SBCD byte predecrement path with A7 and A7.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    sbcd -(a7),-(a7)
    move.l d7,d0
    sbcd -(a7),-(a7)
    move.l d7,d0
    sbcd -(a7),-(a7)
    move.l d7,d0
    sbcd -(a7),-(a7)
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
