; Move l operands from stack_predecrement to data_register.
source_data:
    dcb.w 200000,$5a3c
entry:
    rept {{instruction_count}}
    move.l -(a7),d5
    move.l d3,d4
    move.l -(a7),d5
    move.l d3,d4
    move.l -(a7),d5
    move.l d3,d4
    move.l -(a7),d5
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
