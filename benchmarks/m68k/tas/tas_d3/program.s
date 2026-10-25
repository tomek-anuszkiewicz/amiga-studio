; Four TAS D3 targets, each followed by a MOVE helper.
entry:
    rept {{instruction_count}}
    tas d3
    move.l d6,d7
    tas d3
    move.l d6,d7
    tas d3
    move.l d6,d7
    tas d3
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
