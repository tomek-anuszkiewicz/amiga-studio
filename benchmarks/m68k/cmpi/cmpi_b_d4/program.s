; Measure CMPI.B immediate operand with this destination mode.
entry:
    rept {{instruction_count}}
    cmpi.b #$57,d4
    move.l d7,d0
    cmpi.b #$57,d4
    move.l d7,d0
    cmpi.b #$57,d4
    move.l d7,d0
    cmpi.b #$57,d4
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
