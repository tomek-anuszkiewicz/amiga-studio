; Measure TST.W with the selected operand mode.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    tst.w -(a7)
    move.l d7,d0
    tst.w -(a7)
    move.l d7,d0
    tst.w -(a7)
    move.l d7,d0
    tst.w -(a7)
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
