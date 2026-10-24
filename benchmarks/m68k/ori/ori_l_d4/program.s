; Reload D4 from D7 before each logical target.
entry:
    rept {{instruction_count}}
    ori.l #$13572469,d4
    move.l d7,d4
    ori.l #$13572469,d4
    move.l d7,d4
    ori.l #$13572469,d4
    move.l d7,d4
    ori.l #$13572469,d4
    move.l d7,d4
    endr
    dc.b "M68K_END_MARKER!"
