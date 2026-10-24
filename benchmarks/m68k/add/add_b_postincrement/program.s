; Measure the ADD.B source effective address with four interleaved targets.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    add.b (a1)+,d5
    move.l d7,d0
    add.b (a1)+,d5
    move.l d7,d0
    add.b (a1)+,d5
    move.l d7,d0
    add.b (a1)+,d5
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
