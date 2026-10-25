; Restore A6 after each ADDA target; source EA remains in range.
entry:
    rept {{instruction_count}}
    adda.w #$1234,a6
    movea.l d7,a6
    adda.w #$1234,a6
    movea.l d7,a6
    adda.w #$1234,a6
    movea.l d7,a6
    adda.w #$1234,a6
    movea.l d7,a6
    endr
    dc.b "M68K_END_MARKER!"
