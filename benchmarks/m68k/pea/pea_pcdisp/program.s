; Four PEA *+4(pc) targets accumulate effective addresses on SSP.
entry:
    rept {{instruction_count}}
    pea *+4(pc)
    move.l d6,d7
    pea *+4(pc)
    move.l d6,d7
    pea *+4(pc)
    move.l d6,d7
    pea *+4(pc)
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
