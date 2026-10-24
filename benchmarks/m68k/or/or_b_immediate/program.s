; Reload register destinations from D7 so each logical target uses a fresh operand.
entry:
    rept {{instruction_count}}
    or.b #$35,d0
    move.l d7,d0
    or.b #$35,d0
    move.l d7,d0
    or.b #$35,d0
    move.l d7,d0
    or.b #$35,d0
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
