; Measure SUBQ.B #5 with this destination mode.
entry:
    rept {{instruction_count}}
    subq.b #5,d4
    move.l d7,d0
    subq.b #5,d4
    move.l d7,d0
    subq.b #5,d4
    move.l d7,d0
    subq.b #5,d4
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
