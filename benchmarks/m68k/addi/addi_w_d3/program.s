; Repeat the register form four times with a MOVE helper after each target.
entry:
    rept {{instruction_count}}
    addi.w #$1357,d3
    move.l d7,d0
    addi.w #$1357,d3
    move.l d7,d0
    addi.w #$1357,d3
    move.l d7,d0
    addi.w #$1357,d3
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
