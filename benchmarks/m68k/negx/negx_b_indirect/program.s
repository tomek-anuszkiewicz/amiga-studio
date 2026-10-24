; Measure NEGX.B with the selected destination mode.
operand_data:
    dc.w $3457
entry:
    rept {{instruction_count}}
    negx.b (a3)
    move.l d7,d0
    negx.b (a3)
    move.l d7,d0
    negx.b (a3)
    move.l d7,d0
    negx.b (a3)
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
