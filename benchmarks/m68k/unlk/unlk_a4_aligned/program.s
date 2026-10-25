; Aligned UNLK A4 frames restore the same register value; MOVE.L separates targets.
entry:
    rept {{instruction_count}}
    unlk a4
    move.l d6,d7
    unlk a4
    move.l d6,d7
    unlk a4
    move.l d6,d7
    unlk a4
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
