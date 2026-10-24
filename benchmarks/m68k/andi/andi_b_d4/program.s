; Reload D4 from D7 before each logical target.
entry:
    rept {{instruction_count}}
    andi.b #$57,d4
    move.l d7,d4
    andi.b #$57,d4
    move.l d7,d4
    andi.b #$57,d4
    move.l d7,d4
    andi.b #$57,d4
    move.l d7,d4
    endr
    dc.b "M68K_END_MARKER!"
