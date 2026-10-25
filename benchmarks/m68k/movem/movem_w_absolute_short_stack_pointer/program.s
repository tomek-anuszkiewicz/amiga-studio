; Transfer 1 w register(s) through absolute_short.
entry:
    rept {{instruction_count}}
    movem.w $1000.w,a7
    movea.l d7,a7
    movem.w $1000.w,a7
    movea.l d7,a7
    movem.w $1000.w,a7
    movea.l d7,a7
    movem.w $1000.w,a7
    movea.l d7,a7
    endr
    dc.b "M68K_END_MARKER!"
