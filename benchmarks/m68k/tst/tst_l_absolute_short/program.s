; Measure TST.L with the selected operand mode.
entry:
    rept {{instruction_count}}
    tst.l $1000.w
    move.l d7,d0
    tst.l $1000.w
    move.l d7,d0
    tst.l $1000.w
    move.l d7,d0
    tst.l $1000.w
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
