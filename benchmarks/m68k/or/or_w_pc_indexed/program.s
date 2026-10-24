; Read the preceding MOVE.L opcode as indexed PC-relative operand data; MOVE restores D6.
    dc.w $2C03
entry:
    rept {{instruction_count}}
    or.w *-10(pc,d4.w),d6
    move.l d3,d6
    or.w *-10(pc,d4.w),d6
    move.l d3,d6
    or.w *-10(pc,d4.w),d6
    move.l d3,d6
    or.w *-10(pc,d4.w),d6
    move.l d3,d6
    endr
    dc.b "M68K_END_MARKER!"
