; Restore the long destination before each three-bit shift or rotate.
entry:
    rept {{instruction_count}}
    ror.l #3,d4
    move.l d7,d4
    ror.l #3,d4
    move.l d7,d4
    ror.l #3,d4
    move.l d7,d4
    ror.l #3,d4
    move.l d7,d4
    endr
    dc.b "M68K_END_MARKER!"
