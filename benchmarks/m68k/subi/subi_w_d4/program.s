; Subtract a nontrivial immediate word from a data register.
entry:
    rept {{instruction_count}}
    subi.w #$1357,d4
    move.l d7,d0
    subi.w #$1357,d4
    move.l d7,d0
    subi.w #$1357,d4
    move.l d7,d0
    subi.w #$1357,d4
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
