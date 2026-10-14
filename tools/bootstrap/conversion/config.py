"""Model/effort selection shared by conversion transports."""

from dataclasses import dataclass

@dataclass(frozen=True)
class StageSelection:
    stage: str
    model: str
    reasoning_effort: str


def selection(config, stage):
    pair = config["llm"]["stages"][stage]
    return StageSelection(stage, pair["model"], pair["reasoning_effort"])
