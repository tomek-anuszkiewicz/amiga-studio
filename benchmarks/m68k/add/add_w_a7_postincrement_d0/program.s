; Read successive word operands through SSP postincrement.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    add.w (a7)+,d0
    move.l d7,d1
    add.w (a7)+,d0
    move.l d7,d1
    add.w (a7)+,d0
    move.l d7,d1
    add.w (a7)+,d0
    move.l d7,d1
    endr
    dc.b "M68K_END_MARKER!"
