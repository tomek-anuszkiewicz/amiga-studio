; Restore the destination so every shift or rotate crosses the sign bit.
entry:
    rept {{instruction_count}}
    lsr.w #3,d4
    move.l d7,d4
    lsr.w #3,d4
    move.l d7,d4
    lsr.w #3,d4
    move.l d7,d4
    lsr.w #3,d4
    move.l d7,d4
    endr
    dc.b "M68K_END_MARKER!"
