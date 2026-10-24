; Measure CMP.B *-2(PC),D5 without changing the destination.
; helper opcode or extension-plus-helper pair at the same effective offset.
    dc.w $2803
entry:
    rept {{instruction_count}}
    cmp.b *-2(pc),d5
    move.l d3,d4
    cmp.b *-2(pc),d5
    move.l d3,d4
    cmp.b *-2(pc),d5
    move.l d3,d4
    cmp.b *-2(pc),d5
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
