; Four LEA (A1),A7 targets, each followed by a MOVE helper.
entry:
    rept {{instruction_count}}
    lea (a1),a7
    move.l d6,d7
    lea (a1),a7
    move.l d6,d7
    lea (a1),a7
    move.l d6,d7
    lea (a1),a7
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
