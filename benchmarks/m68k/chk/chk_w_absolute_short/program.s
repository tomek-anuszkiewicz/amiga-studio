; Keep CHK's register value zero and every sampled upper bound nonnegative.
entry:
    rept {{instruction_count}}
    chk.w $1000.w,d2
    move.l d7,d0
    chk.w $1000.w,d2
    move.l d7,d0
    chk.w $1000.w,d2
    move.l d7,d0
    chk.w $1000.w,d2
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
