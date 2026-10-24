; D3 supplies a shift count of 1; D7 restores the long destination.
entry:
    rept {{instruction_count}}
    asr.l d3,d4
    move.l d7,d4
    asr.l d3,d4
    move.l d7,d4
    asr.l d3,d4
    move.l d7,d4
    asr.l d3,d4
    move.l d7,d4
    endr
    dc.b "M68K_END_MARKER!"
