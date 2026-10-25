; Restore A3 after each address-register quick operation.
entry:
    rept {{instruction_count}}
    subq.l #5,a3
    movea.l d7,a3
    subq.l #5,a3
    movea.l d7,a3
    subq.l #5,a3
    movea.l d7,a3
    subq.l #5,a3
    movea.l d7,a3
    endr
    dc.b "M68K_END_MARKER!"
