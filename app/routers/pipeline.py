from fastapi import APIRouter, HTTPException
from pydantic import BaseModel

from app.services.pipeline_service import run_pipeline

router = APIRouter(prefix="/api", tags=["pipeline"])


class GenerateRequest(BaseModel):
    scale: int = 10000


@router.post("/generate")
def generate_sample(payload: GenerateRequest) -> dict:
    if payload.scale <= 0:
        raise HTTPException(status_code=400, detail="Scale must be greater than 0")
    if payload.scale > 200_000:
        raise HTTPException(status_code=400, detail="Scale too large for demo")

    try:
        return run_pipeline(payload.scale)
    except Exception as exc:
        raise HTTPException(status_code=500, detail=str(exc)) from exc
