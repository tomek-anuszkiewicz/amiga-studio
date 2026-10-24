; Move l operands from address_indexed to data_register.
entry:
    rept {{instruction_count}}
    move.l 6(a1,d6.w),d5
    move.l d3,d4
    move.l 6(a1,d6.w),d5
    move.l d3,d4
    move.l 6(a1,d6.w),d5
    move.l d3,d4
    move.l 6(a1,d6.w),d5
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
