; Four PEA $1000.w targets accumulate effective addresses on SSP.
entry:
    rept {{instruction_count}}
    pea $1000.w
    move.l d6,d7
    pea $1000.w
    move.l d6,d7
    pea $1000.w
    move.l d6,d7
    pea $1000.w
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
