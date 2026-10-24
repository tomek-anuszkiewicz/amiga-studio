; Measure BTST with a data_register bit number and stack_predecrement destination.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    btst d3,-(a7)
    move.l d7,d0
    btst d3,-(a7)
    move.l d7,d0
    btst d3,-(a7)
    move.l d7,d0
    btst d3,-(a7)
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
