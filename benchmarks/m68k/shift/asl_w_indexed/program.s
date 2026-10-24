; The fixed memory word converges to zero during this whole-program workload.
    dc.w $1357
    dc.w $2469
    dc.w $357B
    dc.w $468D
    dc.w $579F
    dc.w $68A1
operand_data:
    dc.w $3457
entry:
    rept {{instruction_count}}
    asl.w 6(a3,d5.w)
    move.l d7,d0
    asl.w 6(a3,d5.w)
    move.l d7,d0
    asl.w 6(a3,d5.w)
    move.l d7,d0
    asl.w 6(a3,d5.w)
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
