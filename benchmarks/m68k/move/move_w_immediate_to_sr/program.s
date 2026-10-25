; Move the selected source word to SR in supervisor mode.
entry:
    rept {{instruction_count}}
    move.w #$2700,sr
    move.w d7,sr
    move.w #$2700,sr
    move.w d7,sr
    move.w #$2700,sr
    move.w d7,sr
    move.w #$2700,sr
    move.w d7,sr
    endr
    dc.b "M68K_END_MARKER!"
