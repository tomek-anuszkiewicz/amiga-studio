; Four TRAPV targets, each followed by a MOVE helper.
entry:
    rept {{instruction_count}}
    trapv
    move.l d6,d7
    trapv
    move.l d6,d7
    trapv
    move.l d6,d7
    trapv
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
