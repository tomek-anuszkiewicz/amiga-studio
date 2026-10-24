; Transfer 3 l register(s) through absolute_short.
entry:
    rept {{instruction_count}}
    movem.l d2/d5/a5,$1000.w
    move.l d7,d0
    movem.l d2/d5/a5,$1000.w
    move.l d7,d0
    movem.l d2/d5/a5,$1000.w
    move.l d7,d0
    movem.l d2/d5/a5,$1000.w
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
