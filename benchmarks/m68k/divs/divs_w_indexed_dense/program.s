; Divide restored positive dividends by nonzero word operands; quotient fits.
    dc.w $1357,$246b,$369d,$48af,$5ac1,$6bd3
operand_data:
    dc.w $7ffd
entry:
    rept {{instruction_count}}
    divs.w 6(a3,d2.w),d5
    move.l d6,d5
    divs.w 6(a3,d2.w),d5
    move.l d6,d5
    divs.w 6(a3,d2.w),d5
    move.l d6,d5
    divs.w 6(a3,d2.w),d5
    move.l d6,d5
    endr
    dc.b "M68K_END_MARKER!"
