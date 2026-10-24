; Add a fixed word to successive SSP predecrement destinations.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    add.w d2,-(a7)
    move.l d7,d1
    add.w d2,-(a7)
    move.l d7,d1
    add.w d2,-(a7)
    move.l d7,d1
    add.w d2,-(a7)
    move.l d7,d1
    endr
    dc.b "M68K_END_MARKER!"
