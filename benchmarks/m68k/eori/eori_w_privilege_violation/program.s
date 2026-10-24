; User-mode target traps; the supervisor handler resumes at its MOVE helper.
privilege_handler:
    addq.l #4,2(a7)
    rte
entry:
    rept {{instruction_count}}
    eori.w #$07ff,sr
    move.l d7,d0
    eori.w #$07ff,sr
    move.l d7,d0
    eori.w #$07ff,sr
    move.l d7,d0
    eori.w #$07ff,sr
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
