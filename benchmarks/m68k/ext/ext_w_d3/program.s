; Four EXT.W D3 targets, each followed by a MOVE helper.
entry:
    rept {{instruction_count}}
    ext.w d3
    move.l d6,d3
    ext.w d3
    move.l d6,d3
    ext.w d3
    move.l d6,d3
    ext.w d3
    move.l d6,d3
    endr
    dc.b "M68K_END_MARKER!"
