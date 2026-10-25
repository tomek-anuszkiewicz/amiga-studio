; Restore A7 after each SUBA target; source EA remains in range.
    dc.w $1357
    dc.w $246B
    dc.w $369D
operand_data:
    dc.l $12345679
entry:
    rept {{instruction_count}}
    suba.l 6(a3),a7
    movea.l d7,a7
    suba.l 6(a3),a7
    movea.l d7,a7
    suba.l 6(a3),a7
    movea.l d7,a7
    suba.l 6(a3),a7
    movea.l d7,a7
    endr
    dc.b "M68K_END_MARKER!"
