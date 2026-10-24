; Divide restored positive dividends by nonzero word operands; quotient fits.
operand_data:
    dc.w $7ffd
entry:
    rept {{instruction_count}}
    divs.w $200000.l,d5
    move.l d6,d5
    divs.w $200000.l,d5
    move.l d6,d5
    divs.w $200000.l,d5
    move.l d6,d5
    divs.w $200000.l,d5
    move.l d6,d5
    endr
    dc.b "M68K_END_MARKER!"
