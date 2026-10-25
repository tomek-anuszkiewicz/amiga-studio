; Restore A6 after each SUBA target; source EA remains in range.
entry:
    rept {{instruction_count}}
    suba.w #$1234,a6
    movea.l d7,a6
    suba.w #$1234,a6
    movea.l d7,a6
    suba.w #$1234,a6
    movea.l d7,a6
    suba.w #$1234,a6
    movea.l d7,a6
    endr
    dc.b "M68K_END_MARKER!"
