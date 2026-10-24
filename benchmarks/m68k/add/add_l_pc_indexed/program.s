; Read the preceding MOVE.L opcode as indexed PC-relative operand data.
    dc.w $2803
entry:
    rept {{instruction_count}}
    add.l *-10(pc,d6.w),d5
    move.l d3,d4
    add.l *-10(pc,d6.w),d5
    move.l d3,d4
    add.l *-10(pc,d6.w),d5
    move.l d3,d4
    add.l *-10(pc,d6.w),d5
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
