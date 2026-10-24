; Divide restored positive dividends by nonzero word operands; quotient fits.
entry:
    rept {{instruction_count}}
    divs.w #$7ffd,d5
    move.l d6,d5
    divs.w #$7ffd,d5
    move.l d6,d5
    divs.w #$7ffd,d5
    move.l d6,d5
    divs.w #$7ffd,d5
    move.l d6,d5
    endr
    dc.b "M68K_END_MARKER!"
