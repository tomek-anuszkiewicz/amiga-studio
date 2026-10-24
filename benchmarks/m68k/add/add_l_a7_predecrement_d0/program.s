; Exercise ADD.L with SSP predecrement over independent operands.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    add.l -(a7),d0
    move.l d7,d1
    add.l -(a7),d0
    move.l d7,d1
    add.l -(a7),d0
    move.l d7,d1
    add.l -(a7),d0
    move.l d7,d1
    endr
    dc.b "M68K_END_MARKER!"
