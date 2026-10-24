; Measure the ADD.B source effective address with four interleaved targets.
operand_data:
    dc.w $1234
entry:
    rept {{instruction_count}}
    add.b (a3),d1
    move.l d7,d0
    add.b (a3),d1
    move.l d7,d0
    add.b (a3),d1
    move.l d7,d0
    add.b (a3),d1
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
