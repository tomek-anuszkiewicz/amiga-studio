; Z stays clear: SEQ writes $00 and SNE writes $FF.
entry:
    rept {{instruction_count}}
    seq d3
    move.l d6,d7
    seq d3
    move.l d6,d7
    seq d3
    move.l d6,d7
    seq d3
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
