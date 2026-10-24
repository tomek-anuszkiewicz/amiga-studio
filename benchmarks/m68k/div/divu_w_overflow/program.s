; Four overflowing dividends use two divisor/destination pairs and register-held resets.
entry:
    rept {{instruction_count}}
    divu.w d0,d1
    move.l d4,d1
    divu.w d2,d3
    move.l d5,d3
    divu.w d0,d1
    move.l d6,d1
    divu.w d2,d3
    move.l d7,d3
    endr
    dc.b "M68K_END_MARKER!"
