; Four JSR $300000.l targets call an executable RTS trampoline and return.
; The RTS is the sole interleaving instruction between JSR targets.
entry:
    rept {{instruction_count}}
    jsr $300000.l
    jsr $300000.l
    jsr $300000.l
    jsr $300000.l
    endr
    dc.b "M68K_END_MARKER!"
