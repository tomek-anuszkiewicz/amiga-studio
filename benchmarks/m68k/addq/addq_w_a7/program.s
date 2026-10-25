; Restore A7 after each address-register quick operation.
entry:
    rept {{instruction_count}}
    addq.w #5,a7
    movea.l d7,a7
    addq.w #5,a7
    movea.l d7,a7
    addq.w #5,a7
    movea.l d7,a7
    addq.w #5,a7
    movea.l d7,a7
    endr
    dc.b "M68K_END_MARKER!"
