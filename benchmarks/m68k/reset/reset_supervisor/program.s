; Four RESET targets, each followed by a MOVE helper.
entry:
    rept {{instruction_count}}
    reset
    move.l d6,d7
    reset
    move.l d6,d7
    reset
    move.l d6,d7
    reset
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
