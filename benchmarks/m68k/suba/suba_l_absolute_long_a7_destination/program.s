; Restore A7 after each SUBA target; source EA remains in range.
operand_data:
    dc.l $12345679
entry:
    rept {{instruction_count}}
    suba.l $200000.l,a7
    movea.l d7,a7
    suba.l $200000.l,a7
    movea.l d7,a7
    suba.l $200000.l,a7
    movea.l d7,a7
    suba.l $200000.l,a7
    movea.l d7,a7
    endr
    dc.b "M68K_END_MARKER!"
