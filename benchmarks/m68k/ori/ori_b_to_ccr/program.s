; Restore SR after each target to keep its supervisor and condition-code input fixed.
entry:
    rept {{instruction_count}}
    ori.b #$11,ccr
    move.w #$2704,sr
    ori.b #$11,ccr
    move.w #$2704,sr
    ori.b #$11,ccr
    move.w #$2704,sr
    ori.b #$11,ccr
    move.w #$2704,sr
    endr
    dc.b "M68K_END_MARKER!"
