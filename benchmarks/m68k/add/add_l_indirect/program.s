; Measure the ADD.L source effective address with four interleaved targets.
operand_data:
    dc.l $12345679
entry:
    rept {{instruction_count}}
    add.l (a3),d1
    move.l d7,d0
    add.l (a3),d1
    move.l d7,d0
    add.l (a3),d1
    move.l d7,d0
    add.l (a3),d1
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
