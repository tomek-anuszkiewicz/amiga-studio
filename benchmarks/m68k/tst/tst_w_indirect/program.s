; Measure TST.W with the selected operand mode.
operand_data:
    dc.w $3457
entry:
    rept {{instruction_count}}
    tst.w (a3)
    move.l d7,d0
    tst.w (a3)
    move.l d7,d0
    tst.w (a3)
    move.l d7,d0
    tst.w (a3)
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
