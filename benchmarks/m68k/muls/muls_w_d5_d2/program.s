; Two signed sources use four register-held destination values.
entry:
    rept {{instruction_count}}
    muls.w d0,d1
    move.l d4,d1
    muls.w d2,d3
    move.l d5,d3
    muls.w d0,d1
    move.l d6,d1
    muls.w d2,d3
    move.l d7,d3
    endr
    dc.b "M68K_END_MARKER!"
