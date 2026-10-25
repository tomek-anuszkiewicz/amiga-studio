; Four TRAPV targets; RTE restores the V bit for the next target.
trapv_handler:
    rte
entry:
    rept {{instruction_count}}
    trapv
    trapv
    trapv
    trapv
    endr
    dc.b "M68K_END_MARKER!"
