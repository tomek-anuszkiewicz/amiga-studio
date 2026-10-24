; Measure NEG.B with the selected destination mode.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    neg.b -(a7)
    move.l d7,d0
    neg.b -(a7)
    move.l d7,d0
    neg.b -(a7)
    move.l d7,d0
    neg.b -(a7)
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
