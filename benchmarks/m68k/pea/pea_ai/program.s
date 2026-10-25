; Four PEA (A1) targets accumulate addresses on the stack.
entry:
    rept {{instruction_count}}
    pea (a1)
    move.l d6,d7
    pea (a1)
    move.l d6,d7
    pea (a1)
    move.l d6,d7
    pea (a1)
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
