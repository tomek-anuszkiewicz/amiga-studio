; Keep CHK's register value zero and every sampled upper bound nonnegative.
operand_data:
    dcb.w 100000,$7fff
entry:
    rept {{instruction_count}}
    chk.w -(a7),d0
    move.l d7,d1
    chk.w -(a7),d0
    move.l d7,d1
    chk.w -(a7),d0
    move.l d7,d1
    chk.w -(a7),d0
    move.l d7,d1
    endr
    dc.b "M68K_END_MARKER!"
