; Four JSR (0,A5,D2.W) targets call an RTS at the image origin and return.
; The RTS is the sole interleaving instruction between JSR targets.
return_target:
    rts
entry:
    rept {{instruction_count}}
    jsr (0,a5,d2.w)
    jsr (0,a5,d2.w)
    jsr (0,a5,d2.w)
    jsr (0,a5,d2.w)
    endr
    dc.b "M68K_END_MARKER!"
