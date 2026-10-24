; Use a legal nontrivial quick value on a data register.
entry:
    rept {{instruction_count}}
    subq.w #7,d6
    move.l d7,d0
    subq.w #7,d6
    move.l d7,d0
    subq.w #7,d6
    move.l d7,d0
    subq.w #7,d6
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
