; Load A6 from the selected source EA, then restore it.
entry:
    rept {{instruction_count}}
    movea.l #$12345679,a6
    movea.l d7,a6
    movea.l #$12345679,a6
    movea.l d7,a6
    movea.l #$12345679,a6
    movea.l d7,a6
    movea.l #$12345679,a6
    movea.l d7,a6
    endr
    dc.b "M68K_END_MARKER!"
