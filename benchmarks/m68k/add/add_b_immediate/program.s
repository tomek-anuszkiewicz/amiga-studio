; Measure the ADD.B source effective address with four interleaved targets.
entry:
    rept {{instruction_count}}
    add.b #$35,d0
    move.l d7,d1
    add.b #$35,d0
    move.l d7,d1
    add.b #$35,d0
    move.l d7,d1
    add.b #$35,d0
    move.l d7,d1
    endr
    dc.b "M68K_END_MARKER!"
