; Repeating EOR with the same source alternates the destination between two values.
entry:
    rept {{instruction_count}}
    eor.l d0,d1
    move.l d6,d7
    eor.l d2,d3
    move.l d6,d7
    eor.l d4,d5
    move.l d6,d7
    eor.l d0,d1
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
