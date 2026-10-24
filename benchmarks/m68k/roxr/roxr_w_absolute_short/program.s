; The fixed memory word cycles through rotate states during this workload.
entry:
    rept {{instruction_count}}
    roxr.w $1000.w
    move.l d7,d0
    roxr.w $1000.w
    move.l d7,d0
    roxr.w $1000.w
    move.l d7,d0
    roxr.w $1000.w
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
