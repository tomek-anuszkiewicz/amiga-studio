; Measure CMP.W *-10(PC,D6.W),D5 without changing the destination.
; helper opcode or extension-plus-helper pair at the same effective offset.
    dc.w $2803
entry:
    rept {{instruction_count}}
    cmp.w *-10(pc,d6.w),d5
    move.l d3,d4
    cmp.w *-10(pc,d6.w),d5
    move.l d3,d4
    cmp.w *-10(pc,d6.w),d5
    move.l d3,d4
    cmp.w *-10(pc,d6.w),d5
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
