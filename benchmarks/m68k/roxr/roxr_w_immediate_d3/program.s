; Repeat the register form four times with a MOVE helper after each target.
entry:
    rept {{instruction_count}}
    roxr.w #3,d3
    move.l d7,d0
    roxr.w #3,d3
    move.l d7,d0
    roxr.w #3,d3
    move.l d7,d0
    roxr.w #3,d3
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
