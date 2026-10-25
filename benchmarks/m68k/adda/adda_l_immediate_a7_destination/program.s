; Restore A7 after each ADDA target; source EA remains in range.
entry:
    rept {{instruction_count}}
    adda.l #$12345679,a7
    movea.l d7,a7
    adda.l #$12345679,a7
    movea.l d7,a7
    adda.l #$12345679,a7
    movea.l d7,a7
    adda.l #$12345679,a7
    movea.l d7,a7
    endr
    dc.b "M68K_END_MARKER!"
