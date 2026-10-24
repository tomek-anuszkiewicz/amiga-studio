; Restore the byte destination before each three-bit arithmetic shift.
entry:
    rept {{instruction_count}}
    asl.b #3,d4
    move.l d7,d4
    asl.b #3,d4
    move.l d7,d4
    asl.b #3,d4
    move.l d7,d4
    asl.b #3,d4
    move.l d7,d4
    endr
    dc.b "M68K_END_MARKER!"
