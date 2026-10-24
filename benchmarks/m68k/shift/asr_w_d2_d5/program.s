; Keep the register shift count fixed and restore the signed destination.
entry:
    rept {{instruction_count}}
    asr.w d0,d1
    move.l d2,d3
    asr.w d2,d3
    move.l d4,d5
    asr.w d4,d5
    move.l d6,d7
    asr.w d6,d7
    move.l d0,d1
    endr
    dc.b "M68K_END_MARKER!"
