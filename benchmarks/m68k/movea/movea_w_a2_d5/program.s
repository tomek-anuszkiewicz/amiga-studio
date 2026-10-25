; Load A6 from the selected source EA, then restore it.
entry:
    rept {{instruction_count}}
    movea.w a0,a6
    movea.l d7,a6
    movea.w a1,a6
    movea.l d7,a6
    movea.w a2,a6
    movea.l d7,a6
    movea.w a3,a6
    movea.l d7,a6
    endr
    dc.b "M68K_END_MARKER!"
