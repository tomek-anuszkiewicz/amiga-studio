; Four JSR $1000.w targets call an executable RTS trampoline and return.
; The RTS is the sole interleaving instruction between JSR targets.
entry:
    rept {{instruction_count}}
    jsr $1000.w
    jsr $1000.w
    jsr $1000.w
    jsr $1000.w
    endr
    dc.b "M68K_END_MARKER!"
