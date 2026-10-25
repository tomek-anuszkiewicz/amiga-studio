entry:
    rept {{instruction_count}}
    move.w sr,6(a2)
    move.l d0,d1
    move.w sr,6(a2)
    move.l d0,d1
    move.w sr,6(a2)
    move.l d0,d1
    move.w sr,6(a2)
    move.l d0,d1
    endr
    dc.b "M68K_END_MARKER!"
