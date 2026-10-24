; Repeat the register form four times with a MOVE helper after each target.
entry:
    rept {{instruction_count}}
    adda.w d3,a4
    move.l d7,d0
    adda.w d3,a4
    move.l d7,d0
    adda.w d3,a4
    move.l d7,d0
    adda.w d3,a4
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
