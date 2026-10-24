; Repeat the register form four times with a MOVE helper after each target.
entry:
    rept {{instruction_count}}
    roxl.w #3,d3
    move.l d7,d0
    roxl.w #3,d3
    move.l d7,d0
    roxl.w #3,d3
    move.l d7,d0
    roxl.w #3,d3
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
