; Measure BCHG with a immediate bit number and address_postincrement destination.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    bchg #5,(a3)+
    move.l d7,d0
    bchg #5,(a3)+
    move.l d7,d0
    bchg #5,(a3)+
    move.l d7,d0
    bchg #5,(a3)+
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
