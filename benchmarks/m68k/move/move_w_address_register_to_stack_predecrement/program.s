; Move w operands from address_register to stack_predecrement.
destination_data:
    dcb.w 100000,$a55a
entry:
    rept {{instruction_count}}
    move.w a0,-(a7)
    move.l d3,d4
    move.w a0,-(a7)
    move.l d3,d4
    move.w a0,-(a7)
    move.l d3,d4
    move.w a0,-(a7)
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
