; The fixed memory word cycles through rotate states during this workload.
operand_data:
    dc.w $3457
entry:
    rept {{instruction_count}}
    roxr.w $200000.l
    move.l d7,d0
    roxr.w $200000.l
    move.l d7,d0
    roxr.w $200000.l
    move.l d7,d0
    roxr.w $200000.l
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
