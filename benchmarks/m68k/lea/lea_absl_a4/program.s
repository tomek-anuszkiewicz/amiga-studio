; Four LEA $300000.l targets, each followed by a MOVE helper.
entry:
    rept {{instruction_count}}
    lea $300000.l,a4
    move.l d6,d7
    lea $300000.l,a4
    move.l d6,d7
    lea $300000.l,a4
    move.l d6,d7
    lea $300000.l,a4
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
