; Exercise AND.B with SSP predecrement over independent operands.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    and.b d2,-(a7)
    move.l d7,d1
    and.b d2,-(a7)
    move.l d7,d1
    and.b d2,-(a7)
    move.l d7,d1
    and.b d2,-(a7)
    move.l d7,d1
    endr
    dc.b "M68K_END_MARKER!"
