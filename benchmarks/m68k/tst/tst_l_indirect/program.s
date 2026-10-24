; Measure TST.L with the selected operand mode.
operand_data:
    dc.l $3457689B
entry:
    rept {{instruction_count}}
    tst.l (a3)
    move.l d7,d0
    tst.l (a3)
    move.l d7,d0
    tst.l (a3)
    move.l d7,d0
    tst.l (a3)
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
