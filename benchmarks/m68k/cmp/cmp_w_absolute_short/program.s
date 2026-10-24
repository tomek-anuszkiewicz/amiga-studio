; Measure CMP.W $1000.W,D2 without changing the destination.
entry:
    rept {{instruction_count}}
    cmp.w $1000.w,d2
    move.l d7,d0
    cmp.w $1000.w,d2
    move.l d7,d0
    cmp.w $1000.w,d2
    move.l d7,d0
    cmp.w $1000.w,d2
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
