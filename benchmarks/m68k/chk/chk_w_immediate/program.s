; Keep CHK's register value zero and every sampled upper bound nonnegative.
entry:
    rept {{instruction_count}}
    chk.w #$7fff,d0
    move.l d7,d1
    chk.w #$7fff,d0
    move.l d7,d1
    chk.w #$7fff,d0
    move.l d7,d1
    chk.w #$7fff,d0
    move.l d7,d1
    endr
    dc.b "M68K_END_MARKER!"
