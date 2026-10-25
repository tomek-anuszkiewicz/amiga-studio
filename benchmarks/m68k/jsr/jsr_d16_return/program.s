; Four JSR 6(A3) targets call an RTS at the image origin and return.
; The RTS is the sole interleaving instruction between JSR targets.
return_target:
    rts
entry:
    rept {{instruction_count}}
    jsr 6(a3)
    jsr 6(a3)
    jsr 6(a3)
    jsr 6(a3)
    endr
    dc.b "M68K_END_MARKER!"
