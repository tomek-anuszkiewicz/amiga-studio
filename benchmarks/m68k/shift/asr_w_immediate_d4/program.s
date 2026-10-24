; Restore the destination so every shift or rotate crosses the sign bit.
entry:
    rept {{instruction_count}}
    asr.w #3,d4
    move.l d7,d4
    asr.w #3,d4
    move.l d7,d4
    asr.w #3,d4
    move.l d7,d4
    asr.w #3,d4
    move.l d7,d4
    endr
    dc.b "M68K_END_MARKER!"
