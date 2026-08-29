
This plan describe what should I do to reach goal to Amiga emulator.

## BLEPGEN

Rewrite it to Rest. Compare Blep curves.

## A500

### We need to start with CPU

Write a cpu related code
Need to go cycle by cycle
Write for one instruction from each group
Run tests agains it from SingleStepTests
Review architecture
Run agents to write rest of cpu code, use SingleStepTests in loop
Refine agentic loop with skills, mcp, subagents, especially for review
Run tests agains Winuae test
Run tests agains vamiga test
Run tests againt  other(?) test
Note: we need to look for lock, so we cannot access memory

I saw some comments under other cpu implementations about special cases. 
Not sure if winuae and SingleStepTests should cover them.
But we can try to run some investigation with agent if our cpu logic need to be refined futher

## Debugger

- disasm - should be done in react front
- breakpoints - on memory address instruction, on read/write, condition
- debug steo
- historical data navigation
- memory view with explaination
	- search, dump range, copy/paste, clear, set etc
- registry view with explaination
## A1200

Later

## Other models

Later