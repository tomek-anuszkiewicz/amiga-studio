; Restore the long destination before each three-bit arithmetic shift.
entry:
    rept {{instruction_count}}
    asl.l #3,d4
    move.l d7,d4
    asl.l #3,d4
    move.l d7,d4
    asl.l #3,d4
    move.l d7,d4
    asl.l #3,d4
    move.l d7,d4
    endr
    dc.b "M68K_END_MARKER!"
