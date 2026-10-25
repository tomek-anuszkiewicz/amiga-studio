; Four JMP *+6(pc) targets skip ILLEGAL sentinels and land on MOVE helpers.
entry:
    rept {{instruction_count}}
    jmp *+6(pc)
    dc.w $4afc
    move.l d6,d7
    jmp *+6(pc)
    dc.w $4afc
    move.l d6,d7
    jmp *+6(pc)
    dc.w $4afc
    move.l d6,d7
    jmp *+6(pc)
    dc.w $4afc
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
