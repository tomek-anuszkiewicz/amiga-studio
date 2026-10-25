; Restore A6 after each SUBA target; source EA remains in range.
entry:
    rept {{instruction_count}}
    suba.l a0,a6
    movea.l d7,a6
    suba.l a1,a6
    movea.l d7,a6
    suba.l a2,a6
    movea.l d7,a6
    suba.l a3,a6
    movea.l d7,a6
    endr
    dc.b "M68K_END_MARKER!"
