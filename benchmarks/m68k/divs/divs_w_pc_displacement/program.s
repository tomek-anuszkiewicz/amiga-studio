; Divide restored positive dividends by nonzero word operands; quotient fits.
; Every addressed word is the same $2a06 helper opcode.
    dc.w $2a06,$2a06,$2a06,$2a06
entry:
    rept {{instruction_count}}
    divs.w *-8(pc),d5
    move.l d6,d5
    divs.w *-8(pc),d5
    move.l d6,d5
    divs.w *-8(pc),d5
    move.l d6,d5
    divs.w *-8(pc),d5
    move.l d6,d5
    endr
    dc.b "M68K_END_MARKER!"
