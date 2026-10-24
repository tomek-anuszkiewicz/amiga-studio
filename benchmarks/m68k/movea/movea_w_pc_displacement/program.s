; Load A6 from the selected source EA, then restore it.
; An inert MOVE.L precedes each address-register restoration.
; helper opcode or extension-plus-helper pair at the same effective offset.
    dc.w $2c47
entry:
    rept {{instruction_count}}
    movea.w *-2(pc),a6
    move.l d7,d0
    movea.l d7,a6
    movea.w *-2(pc),a6
    move.l d7,d0
    movea.l d7,a6
    movea.w *-2(pc),a6
    move.l d7,d0
    movea.l d7,a6
    movea.w *-2(pc),a6
    move.l d7,d0
    movea.l d7,a6
    endr
    dc.b "M68K_END_MARKER!"
