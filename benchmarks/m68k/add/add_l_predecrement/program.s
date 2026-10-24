; Measure the ADD.L source effective address with four interleaved targets.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    add.l -(a6),d0
    move.l d7,d1
    add.l -(a6),d0
    move.l d7,d1
    add.l -(a6),d0
    move.l d7,d1
    add.l -(a6),d0
    move.l d7,d1
    endr
    dc.b "M68K_END_MARKER!"
