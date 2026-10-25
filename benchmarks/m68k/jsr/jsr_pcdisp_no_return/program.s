; Four JSR *+4(pc) targets push return addresses and continue at each MOVE.
entry:
    rept {{instruction_count}}
    jsr *+4(pc)
    move.l d6,d7
    jsr *+4(pc)
    move.l d6,d7
    jsr *+4(pc)
    move.l d6,d7
    jsr *+4(pc)
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
