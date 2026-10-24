; Multiply successive generated words through source predecrement.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    muls.w -(a3),d5
    move.l d6,d5
    muls.w -(a3),d5
    move.l d6,d5
    muls.w -(a3),d5
    move.l d6,d5
    muls.w -(a3),d5
    move.l d6,d5
    endr
    dc.b "M68K_END_MARKER!"
