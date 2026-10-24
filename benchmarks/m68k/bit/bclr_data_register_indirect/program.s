; Measure BCLR with a data_register bit number and address_indirect destination.
operand_data:
    dc.w $3457
entry:
    rept {{instruction_count}}
    bclr d3,(a3)
    move.l d7,d0
    bclr d3,(a3)
    move.l d7,d0
    bclr d3,(a3)
    move.l d7,d0
    bclr d3,(a3)
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
