; Multiply successive generated words through SSP predecrement.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    muls.w -(a7),d5
    move.l d6,d5
    muls.w -(a7),d5
    move.l d6,d5
    muls.w -(a7),d5
    move.l d6,d5
    muls.w -(a7),d5
    move.l d6,d5
    endr
    dc.b "M68K_END_MARKER!"
