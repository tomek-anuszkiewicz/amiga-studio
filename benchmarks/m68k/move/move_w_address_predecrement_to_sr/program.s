; Move the selected source word to SR in supervisor mode.
source_data:
    dcb.w 100000,$2700
entry:
    rept {{instruction_count}}
    move.w -(a1),sr
    move.w d7,sr
    move.w -(a1),sr
    move.w d7,sr
    move.w -(a1),sr
    move.w d7,sr
    move.w -(a1),sr
    move.w d7,sr
    endr
    dc.b "M68K_END_MARKER!"
