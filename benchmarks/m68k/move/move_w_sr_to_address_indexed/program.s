entry:
    rept {{instruction_count}}
    move.w sr,6(a2,d1.w)
    move.l d0,d2
    move.w sr,6(a2,d1.w)
    move.l d0,d2
    move.w sr,6(a2,d1.w)
    move.l d0,d2
    move.w sr,6(a2,d1.w)
    move.l d0,d2
    endr
    dc.b "M68K_END_MARKER!"
