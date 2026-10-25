; Load A7 from the selected source EA, then restore it.
entry:
    rept {{instruction_count}}
    movea.l #$12345679,a7
    movea.l d7,a7
    movea.l #$12345679,a7
    movea.l d7,a7
    movea.l #$12345679,a7
    movea.l d7,a7
    movea.l #$12345679,a7
    movea.l d7,a7
    endr
    dc.b "M68K_END_MARKER!"
