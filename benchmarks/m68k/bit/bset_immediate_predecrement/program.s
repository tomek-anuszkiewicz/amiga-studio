; Measure BSET with a immediate bit number and address_predecrement destination.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    bset #5,-(a3)
    move.l d7,d0
    bset #5,-(a3)
    move.l d7,d0
    bset #5,-(a3)
    move.l d7,d0
    bset #5,-(a3)
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
