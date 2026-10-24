; Test a register word without consuming the condition codes.
entry:
    rept {{instruction_count}}
    tst.w d4
    move.l d7,d0
    tst.w d4
    move.l d7,d0
    tst.w d4
    move.l d7,d0
    tst.w d4
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
