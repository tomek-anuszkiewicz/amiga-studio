; D3 supplies a three-bit count; D7 restores the word destination.
entry:
    rept {{instruction_count}}
    ror.w d3,d4
    move.l d7,d4
    ror.w d3,d4
    move.l d7,d4
    ror.w d3,d4
    move.l d7,d4
    ror.w d3,d4
    move.l d7,d4
    endr
    dc.b "M68K_END_MARKER!"
