from typing import Any

from fleet-client.types import (
    RegisterRequest,
    EnrollmentResult,
)


class AgentEnrollment:
    @staticmethod
    async def enroll(stub: Any, request: RegisterRequest) -> EnrollmentResult:
        """
        Register the agent with the Fleet server.

        Args:
            stub: FleetService gRPC stub.
            request: RegisterRequest.

        Returns:
            EnrollmentResult
        """

        response = await stub.RegisterAgent(request)

        return EnrollmentResult(
            node_id=response.node_id,
            token=response.token,
            config=response.config,
        )
