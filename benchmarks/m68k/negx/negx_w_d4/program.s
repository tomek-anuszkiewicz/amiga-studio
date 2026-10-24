; Negate a register word with extend.
entry:
    rept {{instruction_count}}
    negx.w d4
    move.l d7,d0
    negx.w d4
    move.l d7,d0
    negx.w d4
    move.l d7,d0
    negx.w d4
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
