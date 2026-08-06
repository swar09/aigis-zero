import logging
import os
from enum import Enum


class LogFormat(Enum):
    HUMAN = "human"
    JSON = "json"


def init(log_level="INFO", format=LogFormat.HUMAN):
    level = os.getenv("LOG_LEVEL", log_level)

    if format == LogFormat.HUMAN:
        logging.basicConfig(
            level=level,
            format="%(asctime)s %(levelname)s %(threadName)s %(filename)s:%(lineno)d %(message)s",
        )

    else:
        # TODO: Placeholder for JSON logging
        logging.basicConfig(level=level)
