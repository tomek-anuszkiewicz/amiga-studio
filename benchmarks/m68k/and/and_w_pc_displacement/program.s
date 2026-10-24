; Read the preceding MOVE.L opcode as PC-relative operand data; MOVE restores D5.
    dc.w $2A03
entry:
    rept {{instruction_count}}
    and.w *-4(pc),d5
    move.l d3,d5
    and.w *-4(pc),d5
    move.l d3,d5
    and.w *-4(pc),d5
    move.l d3,d5
    and.w *-4(pc),d5
    move.l d3,d5
    endr
    dc.b "M68K_END_MARKER!"
