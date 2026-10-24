; Measure CMP.B #$35,D0 without changing the destination.
entry:
    rept {{instruction_count}}
    cmp.b #$35,d0
    move.l d7,d1
    cmp.b #$35,d0
    move.l d7,d1
    cmp.b #$35,d0
    move.l d7,d1
    cmp.b #$35,d0
    move.l d7,d1
    endr
    dc.b "M68K_END_MARKER!"
