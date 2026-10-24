; Restore A6 after each ADDA target; source EA remains in range.
; An inert MOVE.L precedes each address-register restoration.
; helper opcode or extension-plus-helper pair at the same effective offset.
    dc.w $60f2,$2c47
entry:
    rept {{instruction_count}}
    adda.l *-12(pc,d6.w),a6
    move.l d7,d0
    movea.l d7,a6
    adda.l *-12(pc,d6.w),a6
    move.l d7,d0
    movea.l d7,a6
    adda.l *-12(pc,d6.w),a6
    move.l d7,d0
    movea.l d7,a6
    adda.l *-12(pc,d6.w),a6
    move.l d7,d0
    movea.l d7,a6
    endr
    dc.b "M68K_END_MARKER!"
