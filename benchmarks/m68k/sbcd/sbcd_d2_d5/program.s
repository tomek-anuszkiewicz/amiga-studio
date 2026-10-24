; Subtract packed decimal register operands with extend.
entry:
    rept {{instruction_count}}
    sbcd d0,d1
    move.l d6,d7
    sbcd d2,d3
    move.l d6,d7
    sbcd d4,d5
    move.l d6,d7
    sbcd d0,d1
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
