; Add a data register word to a memory effective address.
    dc.w $1357
    dc.w $246B
    dc.w $369D
    dc.w $48AF
    dc.w $5AC1
    dc.w $6BD3
operand_data:
    dc.w $1234
entry:
    rept {{instruction_count}}
    add.b d2,6(a3,d5.w)
    move.l d7,d0
    add.b d2,6(a3,d5.w)
    move.l d7,d0
    add.b d2,6(a3,d5.w)
    move.l d7,d0
    add.b d2,6(a3,d5.w)
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
