; Four TAS $1000.W targets exercise the Chip RAM write-suppression path.
entry:
    rept {{instruction_count}}
    tas $1000.w
    move.l d6,d7
    tas $1000.w
    move.l d6,d7
    tas $1000.w
    move.l d6,d7
    tas $1000.w
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
