; Measure TST.L with the selected operand mode.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    tst.l -(a7)
    move.l d7,d0
    tst.l -(a7)
    move.l d7,d0
    tst.l -(a7)
    move.l d7,d0
    tst.l -(a7)
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
