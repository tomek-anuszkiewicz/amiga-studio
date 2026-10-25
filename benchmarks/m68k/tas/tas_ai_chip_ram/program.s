; Four TAS (A4) targets in Chip RAM, each followed by an inert MOVE helper.
entry:
    rept {{instruction_count}}
    tas (a4)
    move.l d6,d7
    tas (a4)
    move.l d6,d7
    tas (a4)
    move.l d6,d7
    tas (a4)
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
