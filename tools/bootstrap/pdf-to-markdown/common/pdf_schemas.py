"""Structured response contracts owned by the existing PDF inference stages."""


def object_schema(properties):
    return {"type": "object", "properties": properties, "required": list(properties), "additionalProperties": False}


STRING = {"type": "string"}
BOOL = {"type": "boolean"}
