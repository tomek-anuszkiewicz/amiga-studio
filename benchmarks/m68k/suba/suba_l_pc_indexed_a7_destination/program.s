; Restore A7 after each SUBA target; source EA remains in range.
; An inert MOVE.L precedes each address-register restoration.
; helper opcode or extension-plus-helper pair at the same effective offset.
    dc.w $60f2,$2e47
entry:
    rept {{instruction_count}}
    suba.l *-12(pc,d6.w),a7
    move.l d7,d0
    movea.l d7,a7
    suba.l *-12(pc,d6.w),a7
    move.l d7,d0
    movea.l d7,a7
    suba.l *-12(pc,d6.w),a7
    move.l d7,d0
    movea.l d7,a7
    suba.l *-12(pc,d6.w),a7
    move.l d7,d0
    movea.l d7,a7
    endr
    dc.b "M68K_END_MARKER!"
