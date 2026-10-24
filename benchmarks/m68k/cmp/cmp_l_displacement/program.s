; Measure CMP.L 6(A3),D2 without changing the destination.
    dc.w $1357
    dc.w $246B
    dc.w $369D
operand_data:
    dc.l $12345679
entry:
    rept {{instruction_count}}
    cmp.l 6(a3),d2
    move.l d7,d0
    cmp.l 6(a3),d2
    move.l d7,d0
    cmp.l 6(a3),d2
    move.l d7,d0
    cmp.l 6(a3),d2
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
