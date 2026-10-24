; Repeated MOVEQ immediate to a data register.
entry:
    rept {{instruction_count}}
    moveq #53,d4
    move.l d7,d0
    moveq #53,d4
    move.l d7,d0
    moveq #53,d4
    move.l d7,d0
    moveq #53,d4
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
