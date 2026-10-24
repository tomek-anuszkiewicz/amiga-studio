; Keep CHK's register value zero and every sampled upper bound nonnegative.
; helper opcode or extension-plus-helper pair at the same effective offset.
    dc.w $2803
entry:
    rept {{instruction_count}}
    chk.w *-2(pc),d5
    move.l d3,d4
    chk.w *-2(pc),d5
    move.l d3,d4
    chk.w *-2(pc),d5
    move.l d3,d4
    chk.w *-2(pc),d5
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
