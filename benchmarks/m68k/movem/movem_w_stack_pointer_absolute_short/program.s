; Transfer 1 w register(s) through absolute_short.
entry:
    rept {{instruction_count}}
    movem.w a7,$1000.w
    move.l d7,d0
    movem.w a7,$1000.w
    move.l d7,d0
    movem.w a7,$1000.w
    move.l d7,d0
    movem.w a7,$1000.w
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
