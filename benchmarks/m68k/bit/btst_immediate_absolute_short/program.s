; Measure BTST with a immediate bit number and absolute_short destination.
entry:
    rept {{instruction_count}}
    btst #5,$1000.w
    move.l d7,d0
    btst #5,$1000.w
    move.l d7,d0
    btst #5,$1000.w
    move.l d7,d0
    btst #5,$1000.w
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
