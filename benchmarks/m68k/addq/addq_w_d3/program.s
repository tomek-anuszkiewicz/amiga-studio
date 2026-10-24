; Repeat the register form four times with a MOVE helper after each target.
entry:
    rept {{instruction_count}}
    addq.w #7,d3
    move.l d7,d0
    addq.w #7,d3
    move.l d7,d0
    addq.w #7,d3
    move.l d7,d0
    addq.w #7,d3
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
