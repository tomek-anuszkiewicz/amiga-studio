; Negate a packed decimal register byte with extend.
entry:
    rept {{instruction_count}}
    nbcd d4
    move.l d7,d0
    nbcd d4
    move.l d7,d0
    nbcd d4
    move.l d7,d0
    nbcd d4
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
