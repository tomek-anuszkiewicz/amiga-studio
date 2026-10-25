; Move the selected source word to SR in supervisor mode.
entry:
    rept {{instruction_count}}
    move.w (a1),sr
    move.w d7,sr
    move.w (a1),sr
    move.w d7,sr
    move.w (a1),sr
    move.w d7,sr
    move.w (a1),sr
    move.w d7,sr
    endr
    dc.b "M68K_END_MARKER!"
