; Restore A7 after each SUBA target; source EA remains in range.
entry:
    rept {{instruction_count}}
    suba.l #$12345679,a7
    movea.l d7,a7
    suba.l #$12345679,a7
    movea.l d7,a7
    suba.l #$12345679,a7
    movea.l d7,a7
    suba.l #$12345679,a7
    movea.l d7,a7
    endr
    dc.b "M68K_END_MARKER!"
