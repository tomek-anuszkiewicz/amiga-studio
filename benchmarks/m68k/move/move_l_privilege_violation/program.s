; User-mode target traps; the supervisor handler resumes at its MOVE helper.
privilege_handler:
    addq.l #2,2(a7)
    rte
entry:
    rept {{instruction_count}}
    move.l a3,usp
    move.l d7,d0
    move.l a3,usp
    move.l d7,d0
    move.l a3,usp
    move.l d7,d0
    move.l a3,usp
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
