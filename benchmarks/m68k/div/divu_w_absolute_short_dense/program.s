; Divide restored positive dividends by nonzero word operands; quotient fits.
entry:
    rept {{instruction_count}}
    divu.w $1000.w,d5
    move.l d6,d5
    divu.w $1000.w,d5
    move.l d6,d5
    divu.w $1000.w,d5
    move.l d6,d5
    divu.w $1000.w,d5
    move.l d6,d5
    endr
    dc.b "M68K_END_MARKER!"
