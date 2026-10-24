; The fixed memory word cycles through rotate states during this workload.
entry:
    rept {{instruction_count}}
    roxl.w $1000.w
    move.l d7,d0
    roxl.w $1000.w
    move.l d7,d0
    roxl.w $1000.w
    move.l d7,d0
    roxl.w $1000.w
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
