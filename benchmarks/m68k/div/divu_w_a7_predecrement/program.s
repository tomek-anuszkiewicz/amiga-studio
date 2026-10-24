; Divide restored positive dividends by nonzero word operands; quotient fits.
operand_data:
    dcb.w 100000,$2a06
entry:
    rept {{instruction_count}}
    divu.w -(a7),d5
    move.l d6,d5
    divu.w -(a7),d5
    move.l d6,d5
    divu.w -(a7),d5
    move.l d6,d5
    divu.w -(a7),d5
    move.l d6,d5
    endr
    dc.b "M68K_END_MARKER!"
