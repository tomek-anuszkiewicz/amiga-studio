; Restore A7 after each ADDA target; source EA remains in range.
; helper opcode or extension-plus-helper pair at the same effective offset.
    dc.w $fffa,$2e47
entry:
    rept {{instruction_count}}
    adda.l *-4(pc),a7
    movea.l d7,a7
    adda.l *-4(pc),a7
    movea.l d7,a7
    adda.l *-4(pc),a7
    movea.l d7,a7
    adda.l *-4(pc),a7
    movea.l d7,a7
    endr
    dc.b "M68K_END_MARKER!"
