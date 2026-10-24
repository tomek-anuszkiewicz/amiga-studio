; Divide four restored positive dividends through the fast valid timing path.
entry:
    rept {{instruction_count}}
    divs.w d2,d4
    move.l d7,d4
    divs.w d2,d4
    move.l d7,d4
    divs.w d2,d4
    move.l d7,d4
    divs.w d2,d4
    move.l d7,d4
    endr
    dc.b "M68K_END_MARKER!"
