; Measure TST.L with the selected operand mode.
entry:
    rept {{instruction_count}}
    tst.l d4
    move.l d7,d0
    tst.l d4
    move.l d7,d0
    tst.l d4
    move.l d7,d0
    tst.l d4
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
