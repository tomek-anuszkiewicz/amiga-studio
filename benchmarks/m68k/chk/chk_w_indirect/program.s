; Keep CHK's register value zero and every sampled upper bound nonnegative.
operand_data:
    dc.w $7fff
entry:
    rept {{instruction_count}}
    chk.w (a3),d1
    move.l d7,d0
    chk.w (a3),d1
    move.l d7,d0
    chk.w (a3),d1
    move.l d7,d0
    chk.w (a3),d1
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
