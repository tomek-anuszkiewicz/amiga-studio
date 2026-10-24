; Measure CMP.L $200000.L,D2 without changing the destination.
operand_data:
    dc.l $12345679
entry:
    rept {{instruction_count}}
    cmp.l $200000.l,d2
    move.l d3,d4
    cmp.l $200000.l,d2
    move.l d3,d4
    cmp.l $200000.l,d2
    move.l d3,d4
    cmp.l $200000.l,d2
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
