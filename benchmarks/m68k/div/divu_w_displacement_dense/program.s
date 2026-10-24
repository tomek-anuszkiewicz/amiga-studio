; Divide restored positive dividends by nonzero word operands; quotient fits.
    dc.w $1357,$246b,$369d
operand_data:
    dc.w $7ffd
entry:
    rept {{instruction_count}}
    divu.w 6(a3),d5
    move.l d6,d5
    divu.w 6(a3),d5
    move.l d6,d5
    divu.w 6(a3),d5
    move.l d6,d5
    divu.w 6(a3),d5
    move.l d6,d5
    endr
    dc.b "M68K_END_MARKER!"
