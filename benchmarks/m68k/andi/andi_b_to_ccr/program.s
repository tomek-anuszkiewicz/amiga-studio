; Restore SR after each target to keep its supervisor and condition-code input fixed.
entry:
    rept {{instruction_count}}
    andi.b #$0f,ccr
    move.w #$271f,sr
    andi.b #$0f,ccr
    move.w #$271f,sr
    andi.b #$0f,ccr
    move.w #$271f,sr
    andi.b #$0f,ccr
    move.w #$271f,sr
    endr
    dc.b "M68K_END_MARKER!"
