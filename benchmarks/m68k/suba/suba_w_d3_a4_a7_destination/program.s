; Restore A7 after each SUBA target; source EA remains in range.
entry:
    rept {{instruction_count}}
    suba.w d4,a7
    movea.l d7,a7
    suba.w d5,a7
    movea.l d7,a7
    suba.w d6,a7
    movea.l d7,a7
    suba.w d7,a7
    movea.l d7,a7
    endr
    dc.b "M68K_END_MARKER!"
