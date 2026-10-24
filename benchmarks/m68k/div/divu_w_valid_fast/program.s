; Divide four restored unsigned dividends through the fast valid timing path.
entry:
    rept {{instruction_count}}
    divu.w d2,d4
    move.l d7,d4
    divu.w d2,d4
    move.l d7,d4
    divu.w d2,d4
    move.l d7,d4
    divu.w d2,d4
    move.l d7,d4
    endr
    dc.b "M68K_END_MARKER!"
