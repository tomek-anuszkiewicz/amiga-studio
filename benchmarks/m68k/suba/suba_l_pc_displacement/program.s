; Restore A6 after each SUBA target; source EA remains in range.
; An inert MOVE.L precedes each address-register restoration.
; helper opcode or extension-plus-helper pair at the same effective offset.
    dc.w $fffa,$2c47
entry:
    rept {{instruction_count}}
    suba.l *-4(pc),a6
    move.l d7,d0
    movea.l d7,a6
    suba.l *-4(pc),a6
    move.l d7,d0
    movea.l d7,a6
    suba.l *-4(pc),a6
    move.l d7,d0
    movea.l d7,a6
    suba.l *-4(pc),a6
    move.l d7,d0
    movea.l d7,a6
    endr
    dc.b "M68K_END_MARKER!"
