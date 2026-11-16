"""Bounded continuation evidence for pending changes against the current HEAD."""
import hashlib
import importlib.metadata
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import sys
import uuid

from impact import language_data_input
from repository import git, TIMING_REPORT
