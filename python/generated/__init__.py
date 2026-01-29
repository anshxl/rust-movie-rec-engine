"""Generated gRPC stubs for the ReelRecs ML scoring service."""

from .recommendations_pb2 import (
    CandidateFeatures,
    ScoreRequest,
    ScoreResponse,
)
from .recommendations_pb2_grpc import (
    MLScorerServicer,
    MLScorerStub,
    add_MLScorerServicer_to_server,
)

__all__ = [
    "CandidateFeatures",
    "ScoreRequest",
    "ScoreResponse",
    "MLScorerServicer",
    "MLScorerStub",
    "add_MLScorerServicer_to_server",
]
