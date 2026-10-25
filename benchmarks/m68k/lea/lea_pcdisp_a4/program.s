; Four LEA *+4(pc) targets, each followed by a MOVE helper.
entry:
    rept {{instruction_count}}
    lea *+4(pc),a4
    move.l d6,d7
    lea *+4(pc),a4
    move.l d6,d7
    lea *+4(pc),a4
    move.l d6,d7
    lea *+4(pc),a4
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
