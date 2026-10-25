; Four TAS 14(A1) targets read and write one Fast RAM byte.
entry:
    rept {{instruction_count}}
    tas 14(a1)
    move.l d6,d7
    tas 14(a1)
    move.l d6,d7
    tas 14(a1)
    move.l d6,d7
    tas 14(a1)
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
