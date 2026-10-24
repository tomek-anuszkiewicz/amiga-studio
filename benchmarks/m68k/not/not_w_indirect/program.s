; Measure NOT.W with the selected destination mode.
operand_data:
    dc.w $3457
entry:
    rept {{instruction_count}}
    not.w (a3)
    move.l d7,d0
    not.w (a3)
    move.l d7,d0
    not.w (a3)
    move.l d7,d0
    not.w (a3)
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
