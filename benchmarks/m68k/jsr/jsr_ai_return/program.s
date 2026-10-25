; Four JSR (A1) targets call an RTS at the image origin and return.
; The RTS is the sole interleaving instruction between JSR targets.
return_target:
    rts
entry:
    rept {{instruction_count}}
    jsr (a1)
    jsr (a1)
    jsr (a1)
    jsr (a1)
    endr
    dc.b "M68K_END_MARKER!"
