; The fixed memory word cycles through rotate states during this workload.
operand_data:
    dc.w $3457
entry:
    rept {{instruction_count}}
    ror.w $200000.l
    move.l d7,d0
    ror.w $200000.l
    move.l d7,d0
    ror.w $200000.l
    move.l d7,d0
    ror.w $200000.l
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
