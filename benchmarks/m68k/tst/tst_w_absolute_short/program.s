; Measure TST.W with the selected operand mode.
entry:
    rept {{instruction_count}}
    tst.w $1000.w
    move.l d7,d0
    tst.w $1000.w
    move.l d7,d0
    tst.w $1000.w
    move.l d7,d0
    tst.w $1000.w
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
