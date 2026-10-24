; Measure BTST with a immediate bit number and address_predecrement destination.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    btst #5,-(a3)
    move.l d7,d0
    btst #5,-(a3)
    move.l d7,d0
    btst #5,-(a3)
    move.l d7,d0
    btst #5,-(a3)
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
