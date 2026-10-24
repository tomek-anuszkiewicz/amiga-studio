; Restore the long destination before each three-bit shift or rotate.
entry:
    rept {{instruction_count}}
    lsr.l #3,d4
    move.l d7,d4
    lsr.l #3,d4
    move.l d7,d4
    lsr.l #3,d4
    move.l d7,d4
    lsr.l #3,d4
    move.l d7,d4
    endr
    dc.b "M68K_END_MARKER!"
