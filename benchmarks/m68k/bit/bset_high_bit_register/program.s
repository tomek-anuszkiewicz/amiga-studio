; Reload D4 from D7 so each register bit operation sees the same operand.
entry:
    rept {{instruction_count}}
    bset #21,d4
    move.l d7,d4
    bset #21,d4
    move.l d7,d4
    bset #21,d4
    move.l d7,d4
    bset #21,d4
    move.l d7,d4
    endr
    dc.b "M68K_END_MARKER!"
