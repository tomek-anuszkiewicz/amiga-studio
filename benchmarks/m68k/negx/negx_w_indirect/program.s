; Measure NEGX.W with the selected destination mode.
operand_data:
    dc.w $3457
entry:
    rept {{instruction_count}}
    negx.w (a3)
    move.l d7,d0
    negx.w (a3)
    move.l d7,d0
    negx.w (a3)
    move.l d7,d0
    negx.w (a3)
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
