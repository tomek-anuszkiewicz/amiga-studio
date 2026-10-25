; Four TAS (14,A5,D2.W) targets, each followed by a MOVE helper.
entry:
    rept {{instruction_count}}
    tas (14,a5,d2.w)
    move.l d6,d7
    tas (14,a5,d2.w)
    move.l d6,d7
    tas (14,a5,d2.w)
    move.l d6,d7
    tas (14,a5,d2.w)
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
