; Restore the destination so every arithmetic shift crosses the sign bit.
entry:
    rept {{instruction_count}}
    asl.w #3,d4
    move.l d7,d4
    asl.w #3,d4
    move.l d7,d4
    asl.w #3,d4
    move.l d7,d4
    asl.w #3,d4
    move.l d7,d4
    endr
    dc.b "M68K_END_MARKER!"
