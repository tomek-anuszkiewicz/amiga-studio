; Move b operands from pc_indexed to data_register.
    dc.w $2803
entry:
    rept {{instruction_count}}
    move.b *-10(pc,d6.w),d5
    move.l d3,d4
    move.b *-10(pc,d6.w),d5
    move.l d3,d4
    move.b *-10(pc,d6.w),d5
    move.l d3,d4
    move.b *-10(pc,d6.w),d5
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
