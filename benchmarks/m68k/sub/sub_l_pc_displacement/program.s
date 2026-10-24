; The first source reads the prefix; later sources read the preceding
; helper opcode or extension-plus-helper pair at the same effective offset.
    dc.w $fffa,$2803
entry:
    rept {{instruction_count}}
    sub.l *-4(pc),d5
    move.l d3,d4
    sub.l *-4(pc),d5
    move.l d3,d4
    sub.l *-4(pc),d5
    move.l d3,d4
    sub.l *-4(pc),d5
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
