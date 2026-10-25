; Four SWAP D3 targets, each followed by a MOVE helper.
entry:
    rept {{instruction_count}}
    swap d3
    move.l d6,d7
    swap d3
    move.l d6,d7
    swap d3
    move.l d6,d7
    swap d3
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
