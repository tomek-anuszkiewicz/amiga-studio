; Measure BCHG with a data_register bit number and absolute_long destination.
operand_data:
    dc.w $3457
entry:
    rept {{instruction_count}}
    bchg d3,$200000.l
    move.l d7,d0
    bchg d3,$200000.l
    move.l d7,d0
    bchg d3,$200000.l
    move.l d7,d0
    bchg d3,$200000.l
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
