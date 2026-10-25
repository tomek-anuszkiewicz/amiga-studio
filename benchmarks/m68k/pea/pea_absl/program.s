; Four PEA $300000.l targets accumulate effective addresses on SSP.
entry:
    rept {{instruction_count}}
    pea $300000.l
    move.l d6,d7
    pea $300000.l
    move.l d6,d7
    pea $300000.l
    move.l d6,d7
    pea $300000.l
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
