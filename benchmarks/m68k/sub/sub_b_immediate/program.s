; Measure the SUB.B source effective address with four interleaved targets.
entry:
    rept {{instruction_count}}
    sub.b #$35,d0
    move.l d7,d1
    sub.b #$35,d0
    move.l d7,d1
    sub.b #$35,d0
    move.l d7,d1
    sub.b #$35,d0
    move.l d7,d1
    endr
    dc.b "M68K_END_MARKER!"
