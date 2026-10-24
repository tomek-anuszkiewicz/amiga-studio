; The first three sources read padding; later sources read preceding MOVE.L opcodes.
; Every addressed word is the same $2a06 helper opcode.
    dc.w $2a06,$2a06,$2a06,$2a06,$2a06,$2a06,$2a06
entry:
    rept {{instruction_count}}
    muls.w *-10(pc,d2.w),d5
    move.l d6,d5
    muls.w *-10(pc,d2.w),d5
    move.l d6,d5
    muls.w *-10(pc,d2.w),d5
    move.l d6,d5
    muls.w *-10(pc,d2.w),d5
    move.l d6,d5
    endr
    dc.b "M68K_END_MARKER!"
