; The fixed memory word converges to zero during this whole-program workload.
operand_data:
    dc.w $3457
entry:
    rept {{instruction_count}}
    asr.w (a3)
    move.l d7,d0
    asr.w (a3)
    move.l d7,d0
    asr.w (a3)
    move.l d7,d0
    asr.w (a3)
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
