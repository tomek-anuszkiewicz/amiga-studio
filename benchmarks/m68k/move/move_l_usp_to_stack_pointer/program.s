; Restore the destination after each privileged USP transfer.
entry:
    rept {{instruction_count}}
    move.l usp,a7
    movea.l d7,a7
    move.l usp,a7
    movea.l d7,a7
    move.l usp,a7
    movea.l d7,a7
    move.l usp,a7
    movea.l d7,a7
    endr
    dc.b "M68K_END_MARKER!"
