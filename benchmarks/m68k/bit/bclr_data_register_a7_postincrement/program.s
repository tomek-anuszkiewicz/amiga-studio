; Measure BCLR with a data_register bit number and stack_postincrement destination.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    bclr d3,(a7)+
    move.l d7,d0
    bclr d3,(a7)+
    move.l d7,d0
    bclr d3,(a7)+
    move.l d7,d0
    bclr d3,(a7)+
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
