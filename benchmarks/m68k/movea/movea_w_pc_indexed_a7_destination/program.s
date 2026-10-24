; Load A7 from the selected source EA, then restore it.
; An inert MOVE.L precedes each address-register restoration.
; helper opcode or extension-plus-helper pair at the same effective offset.
    dc.w $2e47
entry:
    rept {{instruction_count}}
    movea.w *-10(pc,d6.w),a7
    move.l d7,d0
    movea.l d7,a7
    movea.w *-10(pc,d6.w),a7
    move.l d7,d0
    movea.l d7,a7
    movea.w *-10(pc,d6.w),a7
    move.l d7,d0
    movea.l d7,a7
    movea.w *-10(pc,d6.w),a7
    move.l d7,d0
    movea.l d7,a7
    endr
    dc.b "M68K_END_MARKER!"
