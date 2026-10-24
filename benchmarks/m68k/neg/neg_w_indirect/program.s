; Measure NEG.W with the selected destination mode.
operand_data:
    dc.w $3457
entry:
    rept {{instruction_count}}
    neg.w (a3)
    move.l d7,d0
    neg.w (a3)
    move.l d7,d0
    neg.w (a3)
    move.l d7,d0
    neg.w (a3)
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
