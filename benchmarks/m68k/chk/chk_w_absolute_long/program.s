; Keep CHK's register value zero and every sampled upper bound nonnegative.
operand_data:
    dc.w $7fff
entry:
    rept {{instruction_count}}
    chk.w $200000.l,d2
    move.l d3,d4
    chk.w $200000.l,d2
    move.l d3,d4
    chk.w $200000.l,d2
    move.l d3,d4
    chk.w $200000.l,d2
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
