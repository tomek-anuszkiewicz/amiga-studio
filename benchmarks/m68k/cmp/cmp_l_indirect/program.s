; Measure CMP.L (A3),D1 without changing the destination.
operand_data:
    dc.l $12345679
entry:
    rept {{instruction_count}}
    cmp.l (a3),d1
    move.l d7,d0
    cmp.l (a3),d1
    move.l d7,d0
    cmp.l (a3),d1
    move.l d7,d0
    cmp.l (a3),d1
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
