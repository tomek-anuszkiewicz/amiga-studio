; D3 supplies a three-bit count; D7 restores the byte destination.
entry:
    rept {{instruction_count}}
    asr.b d3,d4
    move.l d7,d4
    asr.b d3,d4
    move.l d7,d4
    asr.b d3,d4
    move.l d7,d4
    asr.b d3,d4
    move.l d7,d4
    endr
    dc.b "M68K_END_MARKER!"
