; The target writes a byte at the absolute short address.
entry:
    rept {{instruction_count}}
    svs $1000.w
    move.l d6,d7
    svs $1000.w
    move.l d6,d7
    svs $1000.w
    move.l d6,d7
    svs $1000.w
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
