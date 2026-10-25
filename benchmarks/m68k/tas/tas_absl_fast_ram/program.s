; Four TAS $300000.L targets exercise the Fast RAM write path.
entry:
    rept {{instruction_count}}
    tas $300000.l
    move.l d6,d7
    tas $300000.l
    move.l d6,d7
    tas $300000.l
    move.l d6,d7
    tas $300000.l
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
