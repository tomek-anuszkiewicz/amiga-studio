; Load A7 from the selected source EA, then restore it.
entry:
    rept {{instruction_count}}
    movea.w d4,a7
    movea.l d7,a7
    movea.w d5,a7
    movea.l d7,a7
    movea.w d6,a7
    movea.l d7,a7
    movea.w d7,a7
    movea.l d7,a7
    endr
    dc.b "M68K_END_MARKER!"
