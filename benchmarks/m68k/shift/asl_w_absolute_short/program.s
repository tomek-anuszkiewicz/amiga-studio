; The fixed memory word converges to zero during this whole-program workload.
entry:
    rept {{instruction_count}}
    asl.w $1000.w
    move.l d7,d0
    asl.w $1000.w
    move.l d7,d0
    asl.w $1000.w
    move.l d7,d0
    asl.w $1000.w
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
