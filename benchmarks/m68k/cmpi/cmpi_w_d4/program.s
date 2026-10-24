; Compare an immediate word with a register without branching.
entry:
    rept {{instruction_count}}
    cmpi.w #$1357,d4
    move.l d7,d0
    cmpi.w #$1357,d4
    move.l d7,d0
    cmpi.w #$1357,d4
    move.l d7,d0
    cmpi.w #$1357,d4
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
