; The first source reads the prefix; later sources read the preceding
; helper opcode or extension-plus-helper pair at the same effective offset.
    dc.w $60f2,$2803
entry:
    rept {{instruction_count}}
    and.l *-12(pc,d6.w),d5
    move.l d3,d5
    and.l *-12(pc,d6.w),d5
    move.l d3,d5
    and.l *-12(pc,d6.w),d5
    move.l d3,d5
    and.l *-12(pc,d6.w),d5
    move.l d3,d5
    endr
    dc.b "M68K_END_MARKER!"
