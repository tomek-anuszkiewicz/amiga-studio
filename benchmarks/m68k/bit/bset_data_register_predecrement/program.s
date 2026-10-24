; Measure BSET with a data_register bit number and address_predecrement destination.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    bset d3,-(a3)
    move.l d7,d0
    bset d3,-(a3)
    move.l d7,d0
    bset d3,-(a3)
    move.l d7,d0
    bset d3,-(a3)
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
