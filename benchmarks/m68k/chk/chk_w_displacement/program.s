; Keep CHK's register value zero and every sampled upper bound nonnegative.
    dc.w $7fff
    dc.w $7fff
    dc.w $7fff
operand_data:
    dc.w $7fff
entry:
    rept {{instruction_count}}
    chk.w 6(a3),d2
    move.l d7,d0
    chk.w 6(a3),d2
    move.l d7,d0
    chk.w 6(a3),d2
    move.l d7,d0
    chk.w 6(a3),d2
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
