; Measure BCHG with a data_register bit number and address_postincrement destination.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    bchg d3,(a3)+
    move.l d7,d0
    bchg d3,(a3)+
    move.l d7,d0
    bchg d3,(a3)+
    move.l d7,d0
    bchg d3,(a3)+
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
