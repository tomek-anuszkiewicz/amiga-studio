; Four EXT.L D3 targets, each followed by a MOVE helper.
entry:
    rept {{instruction_count}}
    ext.l d3
    move.l d6,d3
    ext.l d3
    move.l d6,d3
    ext.l d3
    move.l d6,d3
    ext.l d3
    move.l d6,d3
    endr
    dc.b "M68K_END_MARKER!"
