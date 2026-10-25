; Restore the destination after each privileged USP transfer.
entry:
    rept {{instruction_count}}
    move.l usp,a3
    movea.l d7,a3
    move.l usp,a3
    movea.l d7,a3
    move.l usp,a3
    movea.l d7,a3
    move.l usp,a3
    movea.l d7,a3
    endr
    dc.b "M68K_END_MARKER!"
