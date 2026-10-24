; Measure NBCD memory addressing with packed decimal operands.
entry:
    rept {{instruction_count}}
    nbcd $1000.w
    move.l d7,d0
    nbcd $1000.w
    move.l d7,d0
    nbcd $1000.w
    move.l d7,d0
    nbcd $1000.w
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
