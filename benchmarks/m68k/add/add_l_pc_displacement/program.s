; Read the preceding MOVE.L opcode as the PC-relative operand.
    dc.w $2803
entry:
    rept {{instruction_count}}
    add.l *-4(pc),d5
    move.l d3,d4
    add.l *-4(pc),d5
    move.l d3,d4
    add.l *-4(pc),d5
    move.l d3,d4
    add.l *-4(pc),d5
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
