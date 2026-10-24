; Measure TST.W with the selected operand mode.
    dc.w $1357
    dc.w $2469
    dc.w $357B
    dc.w $468D
    dc.w $579F
    dc.w $68A1
operand_data:
    dc.w $3457
entry:
    rept {{instruction_count}}
    tst.w 6(a3,d5.w)
    move.l d7,d0
    tst.w 6(a3,d5.w)
    move.l d7,d0
    tst.w 6(a3,d5.w)
    move.l d7,d0
    tst.w 6(a3,d5.w)
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
