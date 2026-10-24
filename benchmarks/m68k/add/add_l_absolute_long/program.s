; Measure the ADD.L source effective address with four interleaved targets.
operand_data:
    dc.l $12345679
entry:
    rept {{instruction_count}}
    add.l $200000.l,d2
    move.l d3,d4
    add.l $200000.l,d2
    move.l d3,d4
    add.l $200000.l,d2
    move.l d3,d4
    add.l $200000.l,d2
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
