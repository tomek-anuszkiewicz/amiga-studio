; Move b operands from stack_predecrement to data_register.
source_data:
    dcb.w 100000,$5a3c
entry:
    rept {{instruction_count}}
    move.b -(a7),d5
    move.l d3,d4
    move.b -(a7),d5
    move.l d3,d4
    move.b -(a7),d5
    move.l d3,d4
    move.b -(a7),d5
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
