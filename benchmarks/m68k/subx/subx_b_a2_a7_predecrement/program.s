; Read ordered source and destination operands backward inside the restored data array.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    subx.b -(a2),-(a7)
    move.l d7,d0
    subx.b -(a2),-(a7)
    move.l d7,d0
    subx.b -(a2),-(a7)
    move.l d7,d0
    subx.b -(a2),-(a7)
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
