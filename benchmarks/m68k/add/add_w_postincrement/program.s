; Measure the ADD.W source effective address with four interleaved targets.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    add.w (a1)+,d5
    move.l d7,d0
    add.w (a1)+,d5
    move.l d7,d0
    add.w (a1)+,d5
    move.l d7,d0
    add.w (a1)+,d5
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
