; Transfer 1 l register(s) through pc_indexed.
; An inert MOVE.L precedes each address-register restoration.
    dc.w $2e47
entry:
    rept {{instruction_count}}
    movem.l *-10(pc,d6.w),a7
    move.l d0,d1
    movea.l d7,a7
    movem.l *-10(pc,d6.w),a7
    move.l d0,d1
    movea.l d7,a7
    movem.l *-10(pc,d6.w),a7
    move.l d0,d1
    movea.l d7,a7
    movem.l *-10(pc,d6.w),a7
    move.l d0,d1
    movea.l d7,a7
    endr
    dc.b "M68K_END_MARKER!"
