; Measure CMP.B $200000.L,D2 without changing the destination.
operand_data:
    dc.w $1234
entry:
    rept {{instruction_count}}
    cmp.b $200000.l,d2
    move.l d3,d4
    cmp.b $200000.l,d2
    move.l d3,d4
    cmp.b $200000.l,d2
    move.l d3,d4
    cmp.b $200000.l,d2
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
