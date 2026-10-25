; Load A7 from the selected source EA, then restore it.
entry:
    rept {{instruction_count}}
    movea.w #$1234,a7
    movea.l d7,a7
    movea.w #$1234,a7
    movea.l d7,a7
    movea.w #$1234,a7
    movea.l d7,a7
    movea.w #$1234,a7
    movea.l d7,a7
    endr
    dc.b "M68K_END_MARKER!"
