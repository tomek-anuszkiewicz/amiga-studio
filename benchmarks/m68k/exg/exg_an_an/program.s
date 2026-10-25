; Four EXG A2,A4 targets, each followed by a MOVE helper.
entry:
    rept {{instruction_count}}
    exg a2,a4
    move.l d6,d7
    exg a2,a4
    move.l d6,d7
    exg a2,a4
    move.l d6,d7
    exg a2,a4
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
