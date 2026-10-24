; The fixed memory word cycles through rotate states during this workload.
entry:
    rept {{instruction_count}}
    ror.w $1000.w
    move.l d7,d0
    ror.w $1000.w
    move.l d7,d0
    ror.w $1000.w
    move.l d7,d0
    ror.w $1000.w
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
