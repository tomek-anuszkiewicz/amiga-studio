; Restore the destination after each privileged USP transfer.
entry:
    rept {{instruction_count}}
    move.l a7,usp
    move.l a4,usp
    move.l a7,usp
    move.l a4,usp
    move.l a7,usp
    move.l a4,usp
    move.l a7,usp
    move.l a4,usp
    endr
    dc.b "M68K_END_MARKER!"
