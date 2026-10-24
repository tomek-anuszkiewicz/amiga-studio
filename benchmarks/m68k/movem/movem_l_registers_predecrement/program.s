; Store three registers into independent longwords through predecrement.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    movem.l d2-d4,-(a3)
    move.l d7,d0
    movem.l d2-d4,-(a3)
    move.l d7,d0
    movem.l d2-d4,-(a3)
    move.l d7,d0
    movem.l d2-d4,-(a3)
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
