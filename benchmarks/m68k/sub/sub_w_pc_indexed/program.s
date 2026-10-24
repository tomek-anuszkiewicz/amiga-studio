; The first source reads the prefix; later sources read the preceding
; helper opcode or extension-plus-helper pair at the same effective offset.
    dc.w $2803
entry:
    rept {{instruction_count}}
    sub.w *-10(pc,d6.w),d5
    move.l d3,d4
    sub.w *-10(pc,d6.w),d5
    move.l d3,d4
    sub.w *-10(pc,d6.w),d5
    move.l d3,d4
    sub.w *-10(pc,d6.w),d5
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
