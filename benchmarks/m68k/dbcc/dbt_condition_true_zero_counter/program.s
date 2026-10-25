; DBT is always true, including when the counter is zero.
; An incorrect branch at the first or last target reaches ILLEGAL.
    dc.w $4AFC
entry:
    rept {{instruction_count}}
    dbt d5,*-2
    move.l d6,d5
    move.w #$0000,ccr
    dbt d5,*+10
    move.l d6,d5
    move.w #$0000,ccr
    dbt d5,*+10
    move.l d6,d5
    move.w #$0000,ccr
    dbt d5,*+26
    move.l d6,d5
    move.w #$0000,ccr
    endr
    dc.b "M68K_END_MARKER!"
    dc.w $4AFC
