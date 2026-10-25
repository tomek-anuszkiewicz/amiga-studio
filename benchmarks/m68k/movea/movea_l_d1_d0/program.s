; Load A6 from the selected source EA, then restore it.
entry:
    rept {{instruction_count}}
    movea.l d0,a6
    movea.l #$00a52000,a6
    movea.l d2,a6
    movea.l #$00a52000,a6
    movea.l d4,a6
    movea.l #$00a52000,a6
    movea.l d0,a6
    movea.l #$00a52000,a6
    endr
    dc.b "M68K_END_MARKER!"
