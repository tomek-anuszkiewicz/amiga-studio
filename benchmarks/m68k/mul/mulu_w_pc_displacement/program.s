; The first two sources read padding; later sources read preceding MOVE.L opcodes.
; Every addressed word is the same $2a06 helper opcode.
    dc.w $2a06,$2a06,$2a06,$2a06
entry:
    rept {{instruction_count}}
    mulu.w *-8(pc),d5
    move.l d6,d5
    mulu.w *-8(pc),d5
    move.l d6,d5
    mulu.w *-8(pc),d5
    move.l d6,d5
    mulu.w *-8(pc),d5
    move.l d6,d5
    endr
    dc.b "M68K_END_MARKER!"
