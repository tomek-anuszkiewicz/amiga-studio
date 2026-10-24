; Measure CMP.L *-12(PC,D6.W),D5 without changing the destination.
; helper opcode or extension-plus-helper pair at the same effective offset.
    dc.w $60f2,$2803
entry:
    rept {{instruction_count}}
    cmp.l *-12(pc,d6.w),d5
    move.l d3,d4
    cmp.l *-12(pc,d6.w),d5
    move.l d3,d4
    cmp.l *-12(pc,d6.w),d5
    move.l d3,d4
    cmp.l *-12(pc,d6.w),d5
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
